use std::collections::HashMap;

use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    system::{Local, Res, ResMut, SystemChangeTick},
    world::World, component::Tick,
};
use tracing::error;

use super::{
    NetworkId, SerializationSettings, SerializedChange, SerializedChangeEventIn, SyncState, Semantics,
};

#[derive(Default)]
pub struct ChangeApplicationState {
    cached_forign_net_ids: HashMap<NetworkId, Entity>,
}

pub fn apply_changes(
    world: &mut World,
    tick: SystemChangeTick,

    mut state: Local<ChangeApplicationState>,
    settings: Res<SerializationSettings>,
    mut sync_state: ResMut<SyncState>,

    mut changes: EventReader<SerializedChangeEventIn>,
) {
    for change in &mut changes {
        match &change.0 {
            SerializedChange::EntitySpawned(net_id) => {
                let entity_id = world.spawn(*net_id).id();

                state.cached_forign_net_ids.insert(*net_id, entity_id);
            }
            SerializedChange::EntityDespawned(net_id) => {
                let Some(entity_id) = state.cached_forign_net_ids.remove(net_id) else {
                    error!("Got remove for unknown or local entity");
                    continue;
                };

                world.despawn(entity_id);
            }
            SerializedChange::ComponentUpdated(net_id, token, Some(serialized)) => {
                let Some(entity_id) = state.cached_forign_net_ids.get(net_id) else {
                    error!("Got update for unknown entity");
                    continue;
                };

                let Some(mut entity) = world.get_entity_mut(*entity_id) else {
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
                    sync_meta.entry(*entity_id).or_insert((Semantics::ForignMutable, Tick::new(0)))
                } else {
                    sync_meta.entry(*entity_id).or_insert((Semantics::LocalMutable, Tick::new(0)))
                };
                sync_meta_entry.1 = tick.this_run();

                // Check if write is allowed
                if sync_meta_entry.0 != Semantics::ForignMutable {
                    error!("Forign modified local controlled component");
                }

                // TODO: error handling
                type_adapter
                    .deserialize(serialized, &mut |ptr| 
                        // SAFETY: We used the type adapter associated with this component id
                        unsafe {
                            entity.insert_by_id(*component_id, ptr);
                        })
                    .expect("Bad update");
            }
            SerializedChange::ComponentUpdated(net_id, token, None) => {
                let Some(entity_id) = state.cached_forign_net_ids.get(net_id) else {
                    error!("Got remove for unknown entity");
                    continue;
                };

                let Some(mut entity) = world.get_entity_mut(*entity_id) else {
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
                    sync_meta.entry(*entity_id).or_insert((Semantics::ForignMutable, Tick::new(0)))
                } else {
                    sync_meta.entry(*entity_id).or_insert((Semantics::LocalMutable, Tick::new(0)))
                };

                // Check if write is allowed
                if sync_meta_entry.0 == Semantics::ForignMutable {
                    sync_meta_entry.1 = tick.this_run();

                    // TODO: there doesnt seem to be a bevy api for this...
                    (remover)(&mut entity);
                } else {
                    error!("Forign removed local controlled component");

                }

            }
            SerializedChange::ResourceUpdated(token, Some(serialized)) => {
                let Some((type_adapter, type_id)) =
                    settings.resource_deserialization.get(token)
                else {
                    error!("Got update for unknown resource token");
                    continue;
                };

                let Some(component_id) =
                    world.components().get_resource_id(*type_id)
                else {
                    error!("Got update for unknown resource");
                    continue;
                };

                // Update the sync meta
                let sync_meta_entry = sync_state.resources.entry(component_id).or_insert((Semantics::ForignMutable, Tick::new(0)));
                sync_meta_entry.1 = tick.this_run();

                // Check if write is allowed
                if sync_meta_entry.0 != Semantics::ForignMutable {
                    error!("Forign modified local controlled resource");
                }

                // TODO: error handling
                type_adapter
                    .deserialize(serialized, &mut |ptr| 
                        // SAFETY: We used the type adapter associated with this component id
                        unsafe {
                            world.insert_resource_by_id(component_id, ptr);
                        })
                    .expect("Bad update");
            }
            SerializedChange::ResourceUpdated(token, None) => {
                let Some((_type_adapter, type_id)) =
                    settings.resource_deserialization.get(token)
                else {
                    error!("Got remove for unknown resource token");
                    continue;
                };

                let Some(component_id) =
                    world.components().get_resource_id(*type_id)
                else {
                    error!("Got remove for unknown resource");
                    continue;
                };

                // Update the sync meta
                let sync_meta_entry = sync_state.resources.entry(component_id).or_insert((Semantics::ForignMutable, Tick::new(0)));

                // Check if write is allowed
                if sync_meta_entry.0 == Semantics::ForignMutable {
                    sync_meta_entry.1 = tick.this_run();

                    world.remove_resource_by_id(component_id);
                } else {
                    error!("Forign modified local controlled resource");
                }
            },
        }
    }
}
