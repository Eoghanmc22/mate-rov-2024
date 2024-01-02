use std::{net::SocketAddr, thread, time::Duration};

use crate::{
    adapters,
    components::Singleton,
    ecs_sync::{
        apply_changes::ChangeApplicationSet, detect_changes::ChangeDetectionSet, EntityMap,
        ForignOwned, NetId, NetTypeId, SerializationSettings, SerializedChange,
        SerializedChangeInEvent, SerializedChangeOutEvent,
    },
    protocol::Protocol,
};
use ahash::{HashMap, HashSet};
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
            .add_event::<ConnectToPeer>()
            .add_event::<DisconnectPeer>()
            .add_event::<SyncPeer>()
            .add_systems(Startup, setup_networking.pipe(error::handle_errors))
            .add_systems(PreUpdate, net_read.before(ChangeApplicationSet))
            .add_systems(
                Update,
                (
                    // ping,
                    flatten_deltas,
                    sync_new_peers.after(flatten_deltas),
                    spawn_peer_entities,
                    disconnect.pipe(error::handle_errors),
                ),
            )
            .add_systems(PostUpdate, net_write.after(ChangeDetectionSet))
            .add_systems(Last, shutdown);

        match self.0 {
            SyncRole::Server => {
                app.add_systems(PostStartup, bind.pipe(error::handle_errors));
            }
            SyncRole::Client => {
                app.add_systems(Update, connect.pipe(error::handle_errors));
            }
        }
    }
}

#[derive(Resource)]
struct Net(Messenger<Protocol>, Receiver<NetEvent<Protocol>>);

#[derive(Resource, Default)]
pub struct Peers {
    by_token: HashMap<NetToken, Entity>,
    by_addrs: HashMap<SocketAddr, Entity>,

    // In bevy time
    pending: HashMap<NetToken, (SocketAddr, Duration)>,

    // TODO: This is kinda bad
    pub(crate) valid_tokens: HashSet<NetToken>,
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
    pub ping: Option<Duration>,
}

#[derive(Event)]
pub struct ConnectToPeer(pub SocketAddr);

#[derive(Event)]
pub struct DisconnectPeer(pub NetToken);

#[derive(Event)]
pub struct SyncPeer(pub NetToken);

fn setup_networking(mut cmds: Commands, errors: Res<Errors>) -> anyhow::Result<()> {
    let networking = Networking::new().context("Start networking")?;
    let handle = networking.messenger();

    let (tx, rx) = channel::bounded(200);

    cmds.insert_resource(Net(handle, rx));

    let errors = errors.0.clone();
    thread::Builder::new()
        .name("Net Thread".to_owned())
        .spawn(move || {
            networking.start(|event| {
                if tx.is_full() {
                    warn!("Not consuming packets fast enough, Network threads will block");

                    let _ = errors.send(anyhow!("Net channel full"));
                }

                // Panicking here isnt terable because it will bring down the net threads if the main
                // app exits uncleanly
                tx.send(event).expect("Channel disconnected");
            })
        })
        .context("Spawn thread")?;

    Ok(())
}

fn bind(net: Res<Net>) -> anyhow::Result<()> {
    net.0
        .bind_at("0.0.0.0:44445".parse().context("Create socket address")?)
        .context("Contact net thread")?;

    Ok(())
}

fn connect(net: Res<Net>, mut events: EventReader<ConnectToPeer>) -> anyhow::Result<()> {
    for event in events.read() {
        net.0.connect_to(event.0).context("Contact net thread")?;
    }

    Ok(())
}

fn disconnect(net: Res<Net>, mut events: EventReader<DisconnectPeer>) -> anyhow::Result<()> {
    for event in events.read() {
        net.0.disconnect(event.0).context("Contact net thread")?;
    }

    Ok(())
}

fn net_read(
    mut cmds: Commands,

    net: Res<Net>,
    time: Res<Time>,

    mut peers: ResMut<Peers>,
    mut entity_map: ResMut<EntityMap>,
    mut changes: EventWriter<SerializedChangeInEvent>,
    mut new_peers: EventWriter<SyncPeer>,

    mut peer_query: Query<(&Peer, &mut Latency)>,

    mut errors: EventWriter<ErrorEvent>,
) {
    for event in net.1.try_iter() {
        match event {
            NetEvent::Conected(token, addrs) | NetEvent::Accepted(token, addrs) => {
                info!(?token, ?addrs, "Peer connected");

                new_peers.send(SyncPeer(token));
                peers.pending.insert(token, (addrs, time.elapsed()));

                peers.valid_tokens.insert(token);
            }
            NetEvent::Data(token, packet) => match packet {
                Protocol::EcsUpdate(update) => {
                    changes.send(SerializedChangeInEvent(update, token));
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
                        .and_then(|it| peer_query.get_component_mut::<Latency>(*it).ok());

                    let Some(mut latency) = latency else {
                        errors.send(anyhow!("Got pong from unknown peer").into());
                        continue;
                    };

                    let sent = Duration::from_millis(payload);
                    latency.last_acknowledged = sent.into();
                    let now = time.elapsed();
                    latency.ping = Some(now - sent);
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
                peers.valid_tokens.remove(&token);

                let Some(entity) = peers.by_token.remove(&token) else {
                    errors.send(anyhow!("Unknown peer disconnected").into());
                    continue;
                };
                let Ok(peer) = peer_query.get_component::<Peer>(entity) else {
                    errors.send(anyhow!("Unknown peer disconnected").into());
                    continue;
                };

                peers.by_addrs.remove(&peer.addrs);

                cmds.entity(entity).despawn();
                if let Some(owned_entities) = entity_map.forign_owned.remove(&token) {
                    for entity in owned_entities {
                        let forign = entity_map.local_to_forign.remove(&entity);
                        if let Some(forign) = forign {
                            entity_map.forign_to_local.remove(&forign);
                        };

                        entity_map.local_modified.remove(&entity);

                        let Some(mut entity) = cmds.get_entity(entity) else {
                            continue;
                        };

                        entity.despawn();
                    }
                }

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

    let rst = net.0.wake();
    if let Err(_) = rst {
        errors.send(anyhow!("Could not wake net thread").into());
    }
}

const SINGLETON_DEADLINE: Duration = Duration::from_millis(100);

fn spawn_peer_entities(
    mut cmds: Commands,
    time: Res<Time>,
    mut peers: ResMut<Peers>,
    query: Query<(Entity, &ForignOwned), Added<Singleton>>,
) {
    let peers = &mut *peers;

    for (entity, owner) in &query {
        let token = NetToken(owner.0);
        let data = peers.pending.remove(&token);

        if let Some((addrs, _)) = data {
            cmds.entity(entity)
                .insert((Peer { addrs, token }, Latency::default()));

            peers.by_token.insert(token, entity);
            peers.by_addrs.insert(addrs, entity);
        }
    }

    let now = time.elapsed();
    peers
        .pending
        .extract_if(|_, (_, time)| now - *time > SINGLETON_DEADLINE)
        .for_each(|(token, (addrs, _))| {
            let entity = cmds.spawn((Peer { addrs, token }, Latency::default())).id();

            peers.by_token.insert(token, entity);
            peers.by_addrs.insert(addrs, entity);
        });
}

fn shutdown(net: Res<Net>, mut exit: EventReader<AppExit>, mut errors: EventWriter<ErrorEvent>) {
    for _event in exit.read() {
        let rst = net.0.shutdown();
        if let Err(_) = rst {
            errors.send(anyhow!("Could not send shutdown event to net thread").into());
        }

        let rst = net.0.wake();
        if let Err(_) = rst {
            errors.send(anyhow!("Could not wake net thread").into());
        }
    }
}

const PING_INTERVAL: Duration = Duration::from_millis(100);
const MAX_LATENCY: Duration = Duration::from_millis(50);

// TODO(high): Auto Reconnect
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
                "Peer at {:?} timed out, now: {:?} lp: {:?}, la: {:?}, elapsed_since: {:?}",
                peer.token,
                now,
                latency.last_ping_sent,
                latency.last_acknowledged,
                latency.last_ping_sent.map(|it| now - it)
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

fn flatten_deltas(
    mut deltas: ResMut<Deltas>,
    entity_map: Res<EntityMap>,

    mut inbound: EventReader<SerializedChangeInEvent>,
    mut outbound: EventReader<SerializedChangeOutEvent>,

    mut errors: EventWriter<ErrorEvent>,
) {
    let iter = Iterator::chain(
        outbound.read().map(|it| &it.0),
        inbound.read().map(|it| &it.0),
    );

    for change in iter {
        match change {
            SerializedChange::EntitySpawned(net_id) => {
                let Some(entity) = entity_map.forign_to_local.get(net_id) else {
                    continue;
                };
                let forign_owned = entity_map
                    .forign_owned
                    .values()
                    .any(|forign_set| forign_set.contains(entity));

                if !forign_owned {
                    deltas.entities.insert(*net_id, HashMap::default());
                }
            }
            SerializedChange::EntityDespawned(net_id) => {
                deltas.entities.remove(net_id);
            }
            SerializedChange::ComponentUpdated(net_id, token, raw) => {
                let Some(entity) = entity_map.forign_to_local.get(net_id) else {
                    continue;
                };
                let forign_owned = entity_map
                    .forign_owned
                    .values()
                    .any(|forign_set| forign_set.contains(entity));

                if !forign_owned {
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
    mut new_peers: EventReader<SyncPeer>,
    mut errors: EventWriter<ErrorEvent>,
) {
    'outer: for &SyncPeer(peer) in new_peers.read() {
        for entity in deltas.entities.keys() {
            let rst = net.0.send_packet(
                peer,
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
                    peer,
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
