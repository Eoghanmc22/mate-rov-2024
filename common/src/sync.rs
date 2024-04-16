use std::{
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs},
    thread,
};

use crate::{
    adapters,
    components::Singleton,
    ecs_sync::{
        apply_changes::ChangeApplicationSet, detect_changes::ChangeDetectionSet, EntityMap,
        ForignOwned, NetId, NetTypeId, SerializationSettings, SerializedChange,
        SerializedChangeInEvent, SerializedChangeOutEvent,
    },
    protocol::Protocol,
    InstanceName,
};
use ahash::{HashMap, HashSet};
use anyhow::{anyhow, Context};
use bevy::{app::AppExit, core::FrameCount, prelude::*};
use crossbeam::channel::{self, Receiver};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use networking::{Event as NetEvent, Messenger, Networking, Token as NetToken};

use crate::error::{self, ErrorEvent, Errors};

const SERVICE_TYPE: &str = "_bevy_ecs_sync._tcp.local.";

pub struct SyncPlugin(pub SyncRole);

#[derive(Resource, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum SyncRole {
    Server { port: u16 },
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
            .insert_resource(self.0)
            .add_event::<ConnectToPeer>()
            .add_event::<DisconnectPeer>()
            .add_event::<SyncPeer>()
            .add_systems(Startup, setup_networking.pipe(error::handle_errors))
            .add_systems(PreUpdate, net_read.before(ChangeApplicationSet))
            .add_systems(
                Update,
                (
                    ping,
                    flatten_deltas,
                    sync_new_peers.after(flatten_deltas),
                    spawn_peer_entities,
                    disconnect.pipe(error::handle_errors),
                ),
            )
            .add_systems(PostUpdate, net_write.after(ChangeDetectionSet))
            .add_systems(Last, shutdown);

        if let SyncRole::Client = self.0 {
            app.add_systems(
                Update,
                (
                    connect.pipe(error::handle_errors),
                    discover_peers.run_if(resource_exists::<MdnsBrowse>),
                ),
            );
        }
    }
}

#[derive(Resource)]
struct Net(Messenger<Protocol>, Receiver<NetEvent<Protocol>>);

#[derive(Resource, Default)]
pub struct Peers {
    by_token: HashMap<NetToken, Entity>,
    by_addrs: HashMap<SocketAddr, Entity>,

    // In frames
    pending: HashMap<NetToken, (SocketAddr, u32)>,

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
    // In frames
    pub last_ping_sent: Option<u32>,
    pub last_acknowledged: Option<u32>,
    pub ping: Option<u32>,
}

#[derive(Resource)]
pub struct MdnsDaemon(ServiceDaemon);

#[derive(Resource)]
pub struct MdnsBrowse(flume::Receiver<ServiceEvent>);

#[derive(Resource, Default)]
pub struct MdnsPeers(pub HashMap<String, DiscoveredPeer>);

pub struct DiscoveredPeer {
    pub info: ServiceInfo,
    pub addresses: Vec<SocketAddr>,
}

#[derive(Event)]
pub struct ConnectToPeer(pub SocketAddr);

#[derive(Event)]
pub struct DisconnectPeer(pub NetToken);

#[derive(Event)]
pub struct SyncPeer(pub NetToken);

fn setup_networking(
    mut cmds: Commands,

    role: Res<SyncRole>,
    name: Res<InstanceName>,

    errors: Res<Errors>,
) -> anyhow::Result<()> {
    info!("Init networking");

    let networking = Networking::new().context("Start networking")?;
    let handle = networking.messenger();

    let (tx, rx) = channel::bounded(200);

    cmds.insert_resource(Net(handle.clone(), rx));

    let errors = errors.0.clone();
    thread::Builder::new()
        .name("Net Thread".to_owned())
        .spawn(move || {
            info!("Starting networking thread");

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

    let mdns = ServiceDaemon::new().context("Could not create mdns daemon")?;

    match &*role {
        SyncRole::Server { port } => {
            // Bind server socket
            let bind = (Ipv4Addr::new(0, 0, 0, 0), *port)
                .to_socket_addrs()
                .context("Resolve bind ip")?
                .next()
                .context("Take first bind ip")?;

            info!("Binding server acceptor");
            handle.bind_at(bind).context("Contact net thread")?;

            // Set up mdns service broadcasting
            let hostname = hostname::get().context("Lookup hostname")?;
            let hostname = hostname.to_str().unwrap();
            let instance_name = &name.0;

            let service_info =
                ServiceInfo::new(SERVICE_TYPE, instance_name, hostname, (), *port, None)
                    .context("Create service info")?
                    .enable_addr_auto();

            info!("Begin broadcasting service");
            mdns.register(service_info)
                .context("Register mdns service")?;
        }
        SyncRole::Client => {
            // Set up mdns service discovery
            info!("Begin searching for services");
            let mdns_events = mdns.browse(SERVICE_TYPE).context("Begin search for peer")?;
            cmds.insert_resource(MdnsBrowse(mdns_events));
            cmds.init_resource::<MdnsPeers>();
        }
    }

    cmds.insert_resource(MdnsDaemon(mdns));

    Ok(())
}

fn connect(net: Res<Net>, mut events: EventReader<ConnectToPeer>) -> anyhow::Result<()> {
    for event in events.read() {
        info!("Connecting to {}", event.0);
        net.0.connect_to(event.0).context("Contact net thread")?;
    }

    Ok(())
}

fn disconnect(net: Res<Net>, mut events: EventReader<DisconnectPeer>) -> anyhow::Result<()> {
    for event in events.read() {
        info!("Disconnecting from {:?}", event.0);
        net.0.disconnect(event.0).context("Contact net thread")?;
    }

    Ok(())
}

fn discover_peers(mut peers: ResMut<MdnsPeers>, browse: Res<MdnsBrowse>) {
    for event in browse.0.try_iter() {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let name = info.get_fullname().split('.').next().unwrap_or("Unknown");
                let host = info.get_hostname();

                info!("Discovered Peer: {}@{}local", name, host);

                let addresses = info
                    .get_addresses_v4()
                    .iter()
                    .flat_map(|address| {
                        (**address, info.get_port())
                            .to_socket_addrs()
                            .into_iter()
                            .flatten()
                    })
                    .collect();

                peers.0.insert(
                    info.get_fullname().to_owned(),
                    DiscoveredPeer { info, addresses },
                );
            }
            ServiceEvent::ServiceRemoved(_, name) => {
                peers.0.remove(&name);
            }
            _ => {}
        }
    }
}

fn net_read(
    mut cmds: Commands,

    net: Res<Net>,
    frame: Res<FrameCount>,

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
                peers.pending.insert(token, (addrs, frame.0));

                peers.valid_tokens.insert(token);
            }
            NetEvent::Data(token, packet) => match packet {
                Protocol::EcsUpdate(update) => {
                    changes.send(SerializedChangeInEvent(update, token));
                }
                Protocol::Ping { payload } => {
                    let response = Protocol::Pong { payload };

                    let rst = net.0.send_packet(token, response);

                    if rst.is_err() {
                        errors.send(anyhow!("Could not reply to ping").into());
                    }
                }
                Protocol::Pong { payload } => {
                    let peer = peers
                        .by_token
                        .get(&token)
                        .and_then(|it| peer_query.get_mut(*it).ok());

                    let Some((_, mut latency)) = peer else {
                        errors.send(anyhow!("Got pong from unknown peer").into());
                        continue;
                    };

                    let sent = payload;
                    let frame = frame.0;

                    latency.last_acknowledged = sent.into();
                    latency.ping = Some(frame.wrapping_sub(sent));
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
                let Ok((peer, _)) = peer_query.get(entity) else {
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

        if rst.is_err() {
            errors.send(anyhow!("Could not brodcast ECS update").into());
        }
    }

    let rst = net.0.wake();
    if rst.is_err() {
        errors.send(anyhow!("Could not wake net thread").into());
    }
}

const SINGLETON_DEADLINE: u32 = 3;

fn spawn_peer_entities(
    mut cmds: Commands,
    frame: Res<FrameCount>,
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

    let frame = frame.0;
    peers
        .pending
        .extract_if(|_, (_, time)| frame.wrapping_sub(*time) > SINGLETON_DEADLINE)
        .for_each(|(token, (addrs, _))| {
            let entity = cmds.spawn((Peer { addrs, token }, Latency::default())).id();

            peers.by_token.insert(token, entity);
            peers.by_addrs.insert(addrs, entity);
        });
}

fn shutdown(
    net: Res<Net>,
    mut exit: EventReader<AppExit>,
    mdns: Option<Res<MdnsDaemon>>,
    mut errors: EventWriter<ErrorEvent>,
) {
    for _event in exit.read() {
        let rst = net.0.shutdown();
        if rst.is_err() {
            errors.send(anyhow!("Could not send shutdown event to net thread").into());
        }

        let rst = net.0.wake();
        if rst.is_err() {
            errors.send(anyhow!("Could not wake net thread").into());
        }

        if let Some(mdns) = &mdns {
            let rst = mdns.0.shutdown();
            if rst.is_err() {
                errors.send(anyhow!("Could not mdns daemon").into());
            }
        }
    }
}

const PING_INTERVAL: u32 = 50;
const MAX_LATENCY: u32 = 15;

// TODO(high): Auto Reconnect
fn ping(
    net: Res<Net>,
    frame: Res<FrameCount>,
    mut query: Query<(&Peer, &mut Latency)>,
    mut errors: EventWriter<ErrorEvent>,
) {
    let frame = frame.0;

    for (peer, mut latency) in &mut query {
        let should_disconnect = match (
            latency.last_ping_sent,
            latency.last_acknowledged,
            latency.ping,
        ) {
            (_, _, Some(ping)) if ping > MAX_LATENCY => true,
            (Some(last_ping), last_ack, _)
                if Some(last_ping) != last_ack && frame.wrapping_sub(last_ping) > MAX_LATENCY =>
            {
                true
            }
            _ => false,
        };

        if should_disconnect {
            error!(
                "Peer at {:?} timed out, now: {:?} lp: {:?}, la: {:?}, elapsed_since: {:?}",
                peer.token,
                frame,
                latency.last_ping_sent,
                latency.last_acknowledged,
                latency.last_ping_sent.map(|it| frame - it)
            );
            let rst = net.0.disconnect(peer.token);

            if rst.is_err() {
                errors.send(anyhow!("Could not disconnect peer").into());
            }
            continue;
        }

        let should_ping = match (latency.last_ping_sent, latency.last_acknowledged) {
            (Some(last_ping), Some(last_ack)) => {
                last_ping == last_ack && frame >= PING_INTERVAL + last_ping
            }
            (Some(_), None) => false,
            _ => true,
        };

        if should_ping {
            let ping = Protocol::Ping { payload: frame };
            let rst = net.0.send_packet(peer.token, ping);

            if rst.is_err() {
                errors.send(anyhow!("Could not send ping").into());
            }

            latency.last_ping_sent = frame.into();
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

            if rst.is_err() {
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

                if rst.is_err() {
                    errors.send(anyhow!("Could not send sync packet").into());
                    continue 'outer;
                }
            }
        }
    }
}
