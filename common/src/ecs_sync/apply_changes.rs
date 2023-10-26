use std::collections::HashMap;

use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    system::{Local, Res, ResMut},
    world::World,
};
use tracing::error;

use super::{
    NetworkId, SerializationSettings, SerializedChange, SerializedChangeEventIn, SyncState,
};

#[derive(Default)]
pub struct ChangeApplicationState {
    cached_forign_net_ids: HashMap<NetworkId, Entity>,
}

pub fn apply_changes(
    world: &mut World,
    // tick: SystemChangeTick,
    //
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
                    error!("Got remove for unknown entity");
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

                let Some((_type_adapter, _id, remover)) =
                    settings.component_deserialization.get(token)
                else {
                    error!("Got remove for unknown component token");
                    continue;
                };

                // TODO: there doesnt seem to be a bevy api for this...
                (remover)(&mut entity);
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

                world.remove_resource_by_id(component_id);
            },
        }
    }
}
