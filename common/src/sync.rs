use std::{
    net::{SocketAddr, ToSocketAddrs},
    thread,
    time::Duration,
};

use crate::{
    adapters,
    ecs_sync::{
        apply_changes::ChangeApplicationSet, detect_changes::ChangeDetectionSet, EntityMap, NetId,
        NetTypeId, SerializationSettings, SerializedChange, SerializedChangeInEvent,
        SerializedChangeOutEvent,
    },
    protocol::Protocol,
};
use ahash::HashMap;
use anyhow::{anyhow, Context};
use bevy::{app::AppExit, prelude::*};
use crossbeam::channel::{self, Receiver};
use networking::{Event as NetEvent, Messenger, Networking, Token as NetToken};

use crate::error::{self, ErrorEvent, Errors};

pub struct SyncPlugin(pub SyncRole);

pub enum SyncRole {
    Server,
    Client,
}

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SerializedChangeInEvent>()
            .add_event::<SerializedChangeOutEvent>()
            .init_resource::<SerializationSettings>()
            .init_resource::<EntityMap>()
            .init_resource::<Deltas>()
            .init_resource::<Peers>()
            .add_systems(Startup, setup_networking.pipe(error::handle_errors))
            .add_systems(PreUpdate, net_read.before(ChangeApplicationSet))
            .add_systems(
                Update,
                (
                    ping,
                    flatten_outbound_deltas,
                    sync_new_peers.after(flatten_outbound_deltas),
                ),
            )
            .add_systems(PostUpdate, net_write.after(ChangeDetectionSet))
            .add_systems(Last, shutdown);

        match self.0 {
            SyncRole::Server => {
                app.add_systems(
                    PostStartup,
                    bind.pipe(error::handle_errors).after(setup_networking),
                );
            }
            SyncRole::Client => {
                app.add_systems(
                    PostStartup,
                    connect.pipe(error::handle_errors).after(setup_networking),
                );
            }
        }
    }
}

#[derive(Resource)]
struct Net(Messenger<Protocol>, Receiver<NetEvent<Protocol>>);

#[derive(Resource, Default)]
struct Peers {
    pub by_token: HashMap<NetToken, Entity>,
    pub by_addrs: HashMap<SocketAddr, Entity>,
}

#[derive(Component, Debug)]
pub struct Peer {
    pub addrs: SocketAddr,
    pub token: NetToken,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct Latency {
    // In bevy time
    pub last_ping_sent: Option<Duration>,
    pub last_acknowledged: Option<Duration>,
}

fn setup_networking(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let networking = Networking::new().context("Start networking")?;
    let handle = networking.messenger();

    let (tx, rx) = channel::bounded(200);

    cmds.insert_resource(Net(handle, rx));

    let errors = errors.0.clone();
    thread::spawn(move || {
        networking.start(|event| {
            if tx.is_full() {
                warn!("Not consuming packets fast enough, Network threads will block");

                let _ = errors.send(anyhow!("Net channel full"));
            }

            // Panicking here isnt terable because it will bring down the net threads if the main
            // app exits uncleanly
            tx.send(event).expect("Channel disconnected");
        })
    });

    Ok(())
}

fn bind(net: Res<Net>) -> anyhow::Result<()> {
    net.0
        .bind_at("0.0.0.0:44445".parse().context("Create socket address")?)
        .context("Contact net thread")?;

    Ok(())
}

fn connect(net: Res<Net>) -> anyhow::Result<()> {
    net.0
        .connect_to(
            "mate.local:44445"
                .to_socket_addrs()
                .context("Create socket address")?
                .next()
                .unwrap(),
        )
        .context("Contact net thread")?;

    Ok(())
}

fn net_read(
    mut cmds: Commands,

    net: Res<Net>,
    mut peers: ResMut<Peers>,
    // mut sync_state: ResMut<SyncState>,
    mut changes: EventWriter<SerializedChangeInEvent>,

    mut query: Query<(&Peer, &mut Latency)>,

    mut errors: EventWriter<ErrorEvent>,
) {
    for event in net.1.try_iter() {
        match event {
            NetEvent::Conected(token, addrs) | NetEvent::Accepted(token, addrs) => {
                info!(?token, ?addrs, "Peer connected");

                let entity = cmds.spawn((Peer { addrs, token }, Latency::default())).id();

                peers.by_token.insert(token, entity);
                peers.by_addrs.insert(addrs, entity);

                // TODO
                // sync_state.singleton_map.insert(token.0, entity);
            }
            NetEvent::Data(token, packet) => match packet {
                Protocol::EcsUpdate(update) => {
                    changes.send(SerializedChangeInEvent(update));
                }
                Protocol::Ping { payload } => {
                    let response = Protocol::Pong { payload };

                    let rst = net.0.send_packet(token, response);

                    if let Err(_) = rst {
                        errors.send(anyhow!("Could not reply to ping").into());
                    }
                }
                Protocol::Pong { payload } => {
                    let latency = peers
                        .by_token
                        .get(&token)
                        .and_then(|it| query.get_component_mut::<Latency>(*it).ok());

                    let Some(mut latency) = latency else {
                        errors.send(anyhow!("Got pong from unknown peer").into());
                        continue;
                    };

                    let sent = Duration::from_millis(payload);
                    latency.last_acknowledged = sent.into();
                }
            },
            NetEvent::Error(token, error) => {
                errors.send(
                    anyhow!(error)
                        .context(format!("Network Error: Token: {token:?}"))
                        .into(),
                );
            }
            NetEvent::Disconnect(token) => {
                let Some(entity) = peers.by_token.remove(&token) else {
                    errors.send(anyhow!("Unknown peer disconnected").into());
                    continue;
                };
                let Ok(peer) = query.get_component::<Peer>(entity) else {
                    errors.send(anyhow!("Unknown peer disconnected").into());
                    continue;
                };

                peers.by_addrs.remove(&peer.addrs);
                // TODO
                // sync_state.singleton_map.remove(&token.0);

                cmds.entity(entity).despawn();

                info!("Peer ({token:?}) at {} disconnected", peer.addrs);
            }
        }
    }
}
fn net_write(
    net: Res<Net>,
    mut changes: EventReader<SerializedChangeOutEvent>,
    mut errors: EventWriter<ErrorEvent>,
) {
    for change in changes.read() {
        let rst = net.0.brodcast_packet(Protocol::EcsUpdate(change.0.clone()));

        if let Err(_) = rst {
            errors.send(anyhow!("Could not brodcast ECS update").into());
        }
    }
}

fn shutdown(net: Res<Net>, mut exit: EventReader<AppExit>, mut errors: EventWriter<ErrorEvent>) {
    for _event in exit.read() {
        let rst = net.0.shutdown();

        if let Err(_) = rst {
            errors.send(anyhow!("Could not send shutdown event to net thread").into());
        }
    }
}

const PING_INTERVAL: Duration = Duration::from_millis(100);
const MAX_LATENCY: Duration = Duration::from_millis(50);

// TODO: Auto Reconnect
fn ping(
    net: Res<Net>,
    time: Res<Time>,
    mut query: Query<(&Peer, &mut Latency)>,
    mut errors: EventWriter<ErrorEvent>,
) {
    let now = time.elapsed();

    for (peer, mut latency) in &mut query {
        let should_disconnect = match (latency.last_ping_sent, latency.last_acknowledged) {
            (Some(last_ping), Some(last_ack)) if last_ack >= last_ping => false,
            (Some(last_ping), _) => now > MAX_LATENCY + last_ping,
            _ => false,
        };

        if should_disconnect {
            error!(
                "Peer at {:?} timed out, now: {:?} lp: {:?}, la: {:?}",
                peer.token, now, latency.last_ping_sent, latency.last_acknowledged
            );
            let rst = net.0.disconnect(peer.token);

            if let Err(_) = rst {
                errors.send(anyhow!("Could not disconnect peer").into());
            }

            continue;
        }

        let should_ping = if let Some(last_ping) = latency.last_ping_sent {
            now > PING_INTERVAL + last_ping
        } else {
            true
        };

        if should_ping {
            let payload = now.as_millis() as u64;
            let ping = Protocol::Ping { payload };
            let rst = net.0.send_packet(peer.token, ping);

            if let Err(_) = rst {
                errors.send(anyhow!("Could not send ping").into());
            }

            latency.last_ping_sent = Duration::from_millis(payload).into();
        }
    }
}

#[derive(Resource, Default, Debug)]
struct Deltas {
    entities: HashMap<NetId, HashMap<NetTypeId, adapters::BackingType>>,
}

fn flatten_outbound_deltas(
    mut deltas: ResMut<Deltas>,
    mut events: EventReader<SerializedChangeOutEvent>,
    mut errors: EventWriter<ErrorEvent>,
) {
    for SerializedChangeOutEvent(change) in events.read() {
        match change {
            SerializedChange::EntitySpawned(net_id) => {
                deltas.entities.insert(*net_id, HashMap::default());
            }
            SerializedChange::EntityDespawned(net_id) => {
                deltas.entities.remove(net_id);
            }
            SerializedChange::ComponentUpdated(net_id, token, raw) => {
                if let Some(components) = deltas.entities.get_mut(net_id) {
                    if let Some(raw) = raw {
                        components.insert(token.clone(), raw.clone());
                    } else {
                        components.remove(token);
                    }
                } else {
                    errors.send(anyhow!("Got bad change event during flattening").into());
                }
            }
            SerializedChange::EventEmitted(_, _) => {
                // New clients should not recieve old events
            }
        }
    }
}

fn sync_new_peers(
    net: Res<Net>,
    deltas: Res<Deltas>,
    query: Query<&Peer, Added<Peer>>,
    mut errors: EventWriter<ErrorEvent>,
) {
    'outer: for peer in query.iter() {
        for entity in deltas.entities.keys() {
            let rst = net.0.send_packet(
                peer.token,
                Protocol::EcsUpdate(SerializedChange::EntitySpawned(*entity)),
            );

            if let Err(_) = rst {
                errors.send(anyhow!("Could not send sync packet").into());
                continue 'outer;
            }
        }

        for (entity, components) in &deltas.entities {
            for (token, raw) in components {
                let rst = net.0.send_packet(
                    peer.token,
                    Protocol::EcsUpdate(SerializedChange::ComponentUpdated(
                        *entity,
                        token.clone(),
                        Some(raw.clone()),
                    )),
                );

                if let Err(_) = rst {
                    errors.send(anyhow!("Could not send sync packet").into());
                    continue 'outer;
                }
            }
        }

        // for (token, raw) in &deltas.resources {
        //     let rst = net.0.send_packet(
        //         peer.token,
        //         Protocol::EcsUpdate(SerializedChange::ResourceUpdated(
        //             token.clone(),
        //             Some(raw.clone()),
        //         )),
        //     );
        //
        //     if let Err(_) = rst {
        //         errors.send(anyhow!("Could not send sync packet").into());
        //         continue 'outer;
        //     }
        // }
    }
}
