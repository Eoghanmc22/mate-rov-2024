use bevy::{
    app::{App, Plugin, PreUpdate},
    ecs::{
        event::EventReader,
        reflect::AppTypeRegistry,
        schedule::{IntoSystemConfigs, SystemSet},
        system::{Commands, Res, ResMut, SystemChangeTick},
        world::{Mut, World},
    },
};
use tracing::error;

use crate::{
    adapters::{dynamic::DynamicAdapter, ComponentTypeAdapter, EventTypeAdapter},
    sync::Peers,
};

use super::{
    EntityMap, ForignOwned, Replicate, SerializationSettings, SerializedChange,
    SerializedChangeInEvent,
};

pub struct ChangeApplicationPlugin;

impl Plugin for ChangeApplicationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, apply_changes.in_set(ChangeApplicationSet));
    }
}

#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ChangeApplicationSet;

fn apply_changes(
    mut cmds: Commands,

    ticks: SystemChangeTick,
    settings: Res<SerializationSettings>,
    mut entity_map: ResMut<EntityMap>,
    peers: Res<Peers>,
    mut reader: EventReader<SerializedChangeInEvent>,
) {
    for SerializedChangeInEvent(change, token) in reader.read() {
        if !peers.valid_tokens.contains(token) {
            // The peer disconnected and has already been cleaned up
            continue;
        }

        match change {
            SerializedChange::EntitySpawned(forign) => {
                let local = cmds.spawn((Replicate, *forign, ForignOwned(token.0))).id();

                entity_map.local_to_forign.insert(local, *forign);
                entity_map.forign_to_local.insert(*forign, local);

                entity_map
                    .forign_owned
                    .entry(*token)
                    .or_default()
                    .insert(local);

                entity_map.local_modified.insert(local, ticks.this_run());
            }
            SerializedChange::EntityDespawned(forign) => {
                let Some(local) = entity_map.forign_to_local.remove(forign) else {
                    error!("Got despawn for unknown entity");
                    continue;
                };

                entity_map.local_to_forign.remove(&local);
                entity_map.local_modified.remove(&local);

                let owned_entities = entity_map.forign_owned.get_mut(token);
                if let Some(owned_entities) = owned_entities {
                    owned_entities.remove(&local);
                }

                cmds.entity(local).despawn();
            }
            SerializedChange::ComponentUpdated(forign, token, Some(serialized)) => {
                let Some(&local) = entity_map.forign_to_local.get(forign) else {
                    error!("Got update for unknown entity");
                    continue;
                };

                let Some(sync_info) = settings.component_by_token.get(token) else {
                    error!("Got update for unknown entity token");
                    continue;
                };

                let type_adapter = sync_info.type_adapter.clone();
                let serialized = serialized.clone();
                let token = token.clone();
                let component_id = sync_info.component_id;

                cmds.add(move |world: &mut World| {
                    // TODO(mid): Error handling
                    match type_adapter {
                        ComponentTypeAdapter::Serde(adapter) => {
                            adapter
                                .deserialize(&serialized, |ptr|
                                    // SAFETY: We used the type adapter associated with this component id
                                    unsafe {
                                        if let Some(mut entity) = world.get_entity_mut(local) {
                                            entity.insert_by_id(component_id, ptr);
                                        }
                                    })
                                .expect("Bad update");
                        }
                        ComponentTypeAdapter::Reflect(_, component) => {
                            world.resource_scope(|world, registry: Mut<AppTypeRegistry>| {
                                let registry = registry.read();

                                let reflect = {
                                    let registration = registry
                                        .get_with_type_path(&token)
                                        .expect("Update for unknown token");

                                    DynamicAdapter::deserialize(
                                        &serialized,
                                        registration,
                                        &registry,
                                    )
                                    .expect("Bad update")
                                };

                                if let Some(mut entity) = world.get_entity_mut(local) {
                                    component.insert(&mut entity, &*reflect, &registry);
                                }
                            })
                        }
                    }
                });

                entity_map.local_modified.insert(local, ticks.this_run());
            }
            SerializedChange::ComponentUpdated(forign, token, None) => {
                let Some(&local) = entity_map.forign_to_local.get(forign) else {
                    error!("Got update for unknown entity");
                    continue;
                };

                let Some(sync_info) = settings.component_by_token.get(token) else {
                    error!("Got update for unknown entity token");
                    continue;
                };

                let remover = sync_info.remove_fn;
                cmds.add(move |world: &mut World| {
                    if let Some(mut entity) = world.get_entity_mut(local) {
                        (remover)(&mut entity);
                    }
                });

                entity_map.local_modified.insert(local, ticks.this_run());
            }
            SerializedChange::EventEmitted(token, serialized) => {
                let Some(sync_info) = settings.event_by_token.get(token) else {
                    error!("Got unknown event");
                    continue;
                };

                let type_adapter = sync_info.type_adapter.clone();
                let serialized = serialized.clone();
                let token = token.clone();

                cmds.add(move |world: &mut World| {
                    // TODO(mid): Error handling
                    match type_adapter {
                        EventTypeAdapter::Serde(adapter, sender) => {
                            adapter
                                .deserialize(&serialized, |ptr|
                                    // SAFETY: We used the type adapter associated with this component id
                                    unsafe {
                                        (sender)(world, ptr)
                                    })
                                .expect("Bad update");
                        }
                        EventTypeAdapter::Reflect(_, event) => {
                            world.resource_scope(|world, registry: Mut<AppTypeRegistry>| {
                                let registry = registry.read();

                                let reflect = {
                                    let registration = registry
                                        .get_with_type_path(&token)
                                        .expect("Update for unknown token");

                                    DynamicAdapter::deserialize(
                                        &serialized,
                                        registration,
                                        &registry,
                                    )
                                    .expect("Bad update")
                                };

                                event.send(world, &*reflect)
                            })
                        }
                    }
                });
            }
        }
    }
}
