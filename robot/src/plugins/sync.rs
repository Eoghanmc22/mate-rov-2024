use std::{net::SocketAddr, thread, time::Duration};

use ahash::HashMap;
use bevy::{app::AppExit, prelude::*};
use common::{
    adapters::BackingType,
    ecs_sync::{
        apply_changes, detect_changes, NetworkId, SerializationSettings, SerializedChange,
        SerializedChangeEventIn, SerializedChangeEventOut, SyncState,
    },
    protocol::Protocol,
    token,
};
use crossbeam::channel::{self, Receiver};
use networking::{Event as NetEvent, Messenger, Networking, Token as NetToken};

pub struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SerializedChangeEventIn>();
        app.add_event::<SerializedChangeEventOut>();

        app.init_resource::<SerializationSettings>();
        app.init_resource::<SyncState>();
        app.init_resource::<Deltas>();

        app.init_resource::<Peers>();

        app.add_systems(Startup, start_server);
        app.add_systems(
            PreUpdate,
            (net_read, apply_changes::apply_changes.after(net_read)),
        );
        app.add_systems(
            Update,
            (
                ping,
                flatten_outbound_deltas,
                sync_new_peers.after(flatten_outbound_deltas),
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                detect_changes::detect_changes.before(net_write),
                net_write,
                shutdown,
            ),
        );
    }
}

#[derive(Resource)]
struct Net(Messenger<Protocol>, Receiver<NetEvent<Protocol>>);

#[derive(Resource, Default)]
pub struct Peers {
    pub by_token: HashMap<NetToken, Entity>,
    pub by_addrs: HashMap<SocketAddr, Entity>,
}

#[derive(Component, Debug)]
pub struct Peer {
    pub addrs: SocketAddr,
    pub token: NetToken,
}

#[derive(Component, Debug, Default)]
pub struct Latency {
    // In bevy time
    pub last_ping_sent: Option<Duration>,
    pub last_acknowledged: Option<Duration>,
}

pub fn start_server(mut cmds: Commands) {
    let networking = Networking::new().expect("Start networking");
    let handle = networking.messenger();

    // TODO: this bound is arbitrary
    let (tx, rx) = channel::bounded(30);

    cmds.insert_resource(Net(handle, rx));

    thread::spawn(move || {
        networking.start(|event| {
            if tx.is_full() {
                error!("Net channel full")
            }

            tx.send(event).expect("Channel disconnected");
        })
    });
}

pub fn net_read(
    mut cmds: Commands,

    net: Res<Net>,
    mut peers: ResMut<Peers>,
    mut sync_state: ResMut<SyncState>,
    mut changes: EventWriter<SerializedChangeEventIn>,

    mut query: Query<(&Peer, &mut Latency)>,
) {
    for event in net.1.try_iter() {
        match event {
            NetEvent::Conected(token, addrs) | NetEvent::Accepted(token, addrs) => {
                let entity = cmds.spawn((Peer { addrs, token }, Latency::default())).id();

                peers.by_token.insert(token, entity);
                peers.by_addrs.insert(addrs, entity);

                sync_state.singleton_map.insert(token.0, entity);
            }
            NetEvent::Data(token, packet) => match packet {
                Protocol::EcsUpdate(update) => {
                    changes.send(SerializedChangeEventIn(update, token.0))
                }
                Protocol::Ping { payload } => {
                    let response = Protocol::Pong { payload };

                    let rst = net.0.send_packet(token, response);

                    if let Err(_) = rst {
                        error!("Could not reply to ping");
                    }
                }
                Protocol::Pong { payload } => {
                    let latency = peers
                        .by_token
                        .get(&token)
                        .and_then(|it| query.get_component_mut::<Latency>(*it).ok());

                    let Some(mut latency) = latency else {
                        error!("Got pong from unknown peer");
                        continue;
                    };

                    let sent = Duration::from_micros(payload);
                    latency.last_acknowledged = sent.into();
                }
            },
            NetEvent::Error(token, error) => {
                error!("Network Error: Token: {token:?} Error: {error:?}");
            }
            NetEvent::Disconnect(token) => {
                let Some(entity) = peers.by_token.remove(&token) else {
                    error!("Unknown peer disconnected");
                    continue;
                };
                let Ok(peer) = query.get_component::<Peer>(entity) else {
                    error!("Unknown peer disconnected");
                    continue;
                };

                peers.by_addrs.remove(&peer.addrs);
                sync_state.singleton_map.remove(&token.0);

                cmds.entity(entity).despawn();

                error!("Peer ({token:?}) at {} disconnected", peer.addrs);
            }
        }
    }
}
pub fn net_write(net: Res<Net>, mut changes: EventReader<SerializedChangeEventOut>) {
    for change in changes.read() {
        let rst = net.0.brodcast_packet(Protocol::EcsUpdate(change.0.clone()));

        if let Err(_) = rst {
            error!("Could not brodcast ECS update");
        }
    }
}

pub fn shutdown(net: Res<Net>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let rst = net.0.shutdown();

        if let Err(_) = rst {
            error!("Could not send shutdown event to net thread");
        }
    }
}

const PING_INTERVAL: Duration = Duration::from_millis(40);
const MAX_LATENCY: Duration = Duration::from_millis(25);

pub fn ping(net: Res<Net>, time: Res<Time>, mut query: Query<(&Peer, &mut Latency)>) {
    let now = time.elapsed();

    for (peer, mut latency) in &mut query {
        let should_disconnect = if let (Some(last_ping), Some(ack)) =
            (latency.last_ping_sent, latency.last_acknowledged)
        {
            last_ping + MAX_LATENCY > ack + PING_INTERVAL
        } else if let Some(last_ping) = latency.last_ping_sent {
            now > MAX_LATENCY + last_ping
        } else {
            false
        };

        if should_disconnect {
            let rst = net.0.disconnect(peer.token);

            if let Err(_) = rst {
                error!("Could not disconnect peer");
            }

            continue;
        }

        let should_ping = if let Some(last_ping) = latency.last_ping_sent {
            now > PING_INTERVAL + last_ping
        } else {
            true
        };

        if should_ping {
            let ping = Protocol::Ping {
                payload: now.as_micros() as u64,
            };
            let rst = net.0.send_packet(peer.token, ping);

            if let Err(_) = rst {
                error!("Could not send ping");
            }

            latency.last_ping_sent = now.into();
        }
    }
}

#[derive(Resource, Default, Debug)]
struct Deltas {
    entities: HashMap<NetworkId, HashMap<token::Key, BackingType>>,
    resources: HashMap<token::Key, BackingType>,
}

pub fn flatten_outbound_deltas(
    mut deltas: ResMut<Deltas>,
    mut events: EventReader<SerializedChangeEventOut>,
) {
    for SerializedChangeEventOut(change) in events.read() {
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
                    error!("Got bad change event during flattening");
                }
            }
            SerializedChange::ResourceUpdated(token, raw) => {
                if let Some(raw) = raw {
                    deltas.resources.insert(token.clone(), raw.clone());
                } else {
                    deltas.resources.remove(token);
                }
            }
        }
    }
}

pub fn sync_new_peers(net: Res<Net>, deltas: Res<Deltas>, query: Query<&Peer, Added<Peer>>) {
    'outer: for peer in query.iter() {
        for entity in deltas.entities.keys() {
            let rst = net.0.send_packet(
                peer.token,
                Protocol::EcsUpdate(SerializedChange::EntitySpawned(*entity)),
            );

            if let Err(_) = rst {
                error!("Could not send sync packet");
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
                    error!("Could not send sync packet");
                    continue 'outer;
                }
            }
        }

        for (token, raw) in &deltas.resources {
            let rst = net.0.send_packet(
                peer.token,
                Protocol::EcsUpdate(SerializedChange::ResourceUpdated(
                    token.clone(),
                    Some(raw.clone()),
                )),
            );

            if let Err(_) = rst {
                error!("Could not send sync packet");
                continue 'outer;
            }
        }
    }
}
