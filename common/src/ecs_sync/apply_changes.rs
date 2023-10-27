use ahash::HashMap;
use bevy_ecs::{
    component::Tick,
    entity::Entity,
    event::{Events, ManualEventReader},
    system::{CommandQueue, Commands, Local, SystemChangeTick, SystemState},
    world::World,
};
use tracing::error;

use super::{
    NetworkId, Semantics, SerializationSettings, SerializedChange, SerializedChangeEventIn,
    SyncState,
};

#[derive(Default)]
pub struct ChangeApplicationState {
    cached_forign_net_ids: HashMap<NetworkId, Entity>,
}

pub fn apply_changes(
    world: &mut World,
    tick: &mut SystemState<SystemChangeTick>,

    mut reader: Local<Option<ManualEventReader<SerializedChangeEventIn>>>,
    mut state: Local<ChangeApplicationState>,
) {
    // Reborrows
    let mut sync_state = world.remove_resource::<SyncState>().unwrap();
    let events = world
        .get_resource::<Events<SerializedChangeEventIn>>()
        .unwrap();
    let settings = world.get_resource::<SerializationSettings>().unwrap();
    let state = &mut *state;

    let reader = reader.get_or_insert_with(|| events.get_reader());

    let tick = tick.get(world);

    let mut queue = CommandQueue::default();
    let mut cmds = Commands::new(&mut queue, world);

    for SerializedChangeEventIn(change, peer) in reader.read(events) {
        match change {
            SerializedChange::EntitySpawned(net_id) => {
                let entity_id = if *net_id == NetworkId::SINGLETON {
                    sync_state.singleton_map.get(peer).cloned()
                } else {
                    None
                };

                let entity_id = entity_id.unwrap_or_else(|| cmds.spawn(*net_id).id());

                state.cached_forign_net_ids.insert(*net_id, entity_id);
            }
            SerializedChange::EntityDespawned(net_id) => {
                let Some(entity_id) = state.cached_forign_net_ids.remove(net_id) else {
                    error!("Got remove for unknown or local entity");
                    continue;
                };

                cmds.entity(entity_id).despawn();
            }
            SerializedChange::ComponentUpdated(net_id, token, Some(serialized)) => {
                let Some(&entity_id) = state.cached_forign_net_ids.get(net_id) else {
                    error!("Got update for unknown entity");
                    continue;
                };

                let Some(entity) = world.get_entity(entity_id) else {
                    error!("Got update for despawned entity");
                    state.cached_forign_net_ids.remove(net_id);
                    continue;
                };

                let Some((type_adapter, component_id, _remover)) =
                    settings.component_deserialization.get(token)
                else {
                    error!("Got update for unknown entity token");
                    continue;
                };

                // Update the sync meta
                let sync_meta = sync_state.components.entry(*component_id).or_default();
                let sync_meta_entry = if !entity.contains_id(*component_id) {
                    sync_meta
                        .entry(entity_id)
                        .or_insert((Semantics::ForignMutable, Tick::new(0)))
                } else {
                    sync_meta
                        .entry(entity_id)
                        .or_insert((Semantics::LocalMutable, Tick::new(0)))
                };
                sync_meta_entry.1 = tick.this_run();

                // Check if write is allowed
                if sync_meta_entry.0 != Semantics::ForignMutable {
                    error!("Forign modified local controlled component");
                }

                let type_adapter = type_adapter.clone();
                let component_id = *component_id;
                // TODO: this will be slow
                let serialized = serialized.clone();
                cmds.add(move |world: &mut World| {
                    // TODO: error handling
                    type_adapter
                        .deserialize(&serialized, &mut |ptr|
                        // SAFETY: We used the type adapter associated with this component id
                        unsafe {
                            world.entity_mut(entity_id).insert_by_id(component_id, ptr);
                        })
                        .expect("Bad update");
                });
            }
            SerializedChange::ComponentUpdated(net_id, token, None) => {
                let Some(&entity_id) = state.cached_forign_net_ids.get(net_id) else {
                    error!("Got remove for unknown entity");
                    continue;
                };

                let Some(entity) = world.get_entity(entity_id) else {
                    error!("Got remove for despawned entity");
                    state.cached_forign_net_ids.remove(net_id);
                    continue;
                };

                let Some((_type_adapter, component_id, remover)) =
                    settings.component_deserialization.get(token)
                else {
                    error!("Got remove for unknown component token");
                    continue;
                };

                // Update the sync meta
                let sync_meta = sync_state.components.entry(*component_id).or_default();
                let sync_meta_entry = if !entity.contains_id(*component_id) {
                    sync_meta
                        .entry(entity_id)
                        .or_insert((Semantics::ForignMutable, Tick::new(0)))
                } else {
                    sync_meta
                        .entry(entity_id)
                        .or_insert((Semantics::LocalMutable, Tick::new(0)))
                };

                // Check if write is allowed
                if sync_meta_entry.0 == Semantics::ForignMutable {
                    sync_meta_entry.1 = tick.this_run();

                    // TODO: there doesnt seem to be a bevy api for this...
                    let remover = *remover;
                    cmds.add(move |world: &mut World| {
                        let mut entity = world.entity_mut(entity_id);
                        (remover)(&mut entity);
                    });
                } else {
                    error!("Forign removed local controlled component");
                }
            }
            SerializedChange::ResourceUpdated(token, Some(serialized)) => {
                let Some((type_adapter, type_id)) = settings.resource_deserialization.get(token)
                else {
                    error!("Got update for unknown resource token");
                    continue;
                };

                let Some(component_id) = world.components().get_resource_id(*type_id) else {
                    error!("Got update for unknown resource");
                    continue;
                };

                // Update the sync meta
                let sync_meta_entry = sync_state
                    .resources
                    .entry(component_id)
                    .or_insert((Semantics::ForignMutable, Tick::new(0)));
                sync_meta_entry.1 = tick.this_run();

                // Check if write is allowed
                if sync_meta_entry.0 != Semantics::ForignMutable {
                    error!("Forign modified local controlled resource");
                }

                let type_adapter = type_adapter.clone();
                let serialized = serialized.clone();
                cmds.add(move |world: &mut World| {
                    // TODO: error handling
                    type_adapter
                        .deserialize(&serialized, &mut |ptr|
                        // SAFETY: We used the type adapter associated with this component id
                        unsafe {
                            world.insert_resource_by_id(component_id, ptr);
                        })
                        .expect("Bad update");
                });
            }
            SerializedChange::ResourceUpdated(token, None) => {
                let Some((_type_adapter, type_id)) = settings.resource_deserialization.get(token)
                else {
                    error!("Got remove for unknown resource token");
                    continue;
                };

                let Some(component_id) = world.components().get_resource_id(*type_id) else {
                    error!("Got remove for unknown resource");
                    continue;
                };

                // Update the sync meta
                let sync_meta_entry = sync_state
                    .resources
                    .entry(component_id)
                    .or_insert((Semantics::ForignMutable, Tick::new(0)));

                // Check if write is allowed
                if sync_meta_entry.0 == Semantics::ForignMutable {
                    sync_meta_entry.1 = tick.this_run();

                    cmds.add(move |world: &mut World| {
                        world.remove_resource_by_id(component_id);
                    })
                } else {
                    error!("Forign modified local controlled resource");
                }
            }
        }
    }

    world.insert_resource(sync_state);
}
