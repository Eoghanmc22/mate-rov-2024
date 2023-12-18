use bevy::app::{App, Plugin, PostUpdate};
use bevy::ecs::schedule::SystemSet;
use bevy::ecs::{
    archetype::ArchetypeId,
    change_detection::DetectChanges,
    component::StorageType,
    entity::Entity,
    event::EventWriter,
    ptr::UnsafeCellDeref,
    query::{Added, With, Without},
    removal_detection::{RemovedComponentEvents, RemovedComponents},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, SystemChangeTick},
    world::{EntityRef, World},
};

use super::{
    EntityMap, NetId, Replicate, SerializationSettings, SerializedChange, SerializedChangeOutEvent,
};

// TODO: Events as RPC
pub struct ChangeDetectionPlugin;

impl Plugin for ChangeDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                detect_new_entities,
                detect_changes.after(detect_new_entities),
                detect_removals.after(detect_changes),
                detect_despawns.after(detect_removals),
            )
                .in_set(ChangeDetectionSet),
        );
    }
}

#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ChangeDetectionSet;

// Detect new entities
// query for added sync component
fn detect_new_entities(
    mut cmds: Commands,
    new_entities: Query<Entity, (Added<Replicate>, Without<NetId>)>,
    mut entity_map: ResMut<EntityMap>,
    mut events: EventWriter<SerializedChangeOutEvent>,
) {
    for entity in &new_entities {
        let remote_entity = *entity_map
            .local_to_forign
            .entry(entity)
            .or_insert_with(NetId::random);
        entity_map.forign_to_local.insert(remote_entity, entity);

        cmds.entity(entity).insert(remote_entity);

        events.send(SerializedChangeOutEvent(SerializedChange::EntitySpawned(
            remote_entity,
        )));
    }
}

// Detect when entities change
// Traverse all archetypes
// filter for the ones we care about
// check for ignore components
// if any non ignored components have changed, sync them
// TODO: Can this be merged with detect new?
fn detect_changes(
    world: &World,
    settings: Res<SerializationSettings>,
    entity_map: Res<EntityMap>,
    ticks: SystemChangeTick,
    mut events: EventWriter<SerializedChangeOutEvent>,
) {
    for archetype in world
        .archetypes()
        .iter()
        .filter(|archetype| archetype.id() != ArchetypeId::EMPTY)
        .filter(|archetype| archetype.id() != ArchetypeId::INVALID)
        .filter(|archetype| archetype.contains(settings.marker_id))
    {
        let table = world
            .storages()
            .tables
            .get(archetype.table_id())
            .expect("Archetype should be valid");

        for entity in archetype.entities() {
            let added = world
                .entity(entity.entity())
                .get_ref::<Replicate>()
                .expect("Has Replicate")
                .is_added();

            for (component_id, sync_info) in archetype
                .components()
                .filter_map(|it| Some(it).zip(settings.tracked_components.get(&it)))
                .filter(|(_, sync_info)| !archetype.contains(sync_info.ignore_component))
            {
                let (ptr, tick) = match archetype
                    .get_storage_type(component_id)
                    .expect("Archatype has component")
                {
                    StorageType::Table => {
                        let column = table
                            .get_column(component_id)
                            .expect("Archatype column has component");

                        column.get(entity.table_row()).expect("Column has entity")
                    }
                    StorageType::SparseSet => {
                        let set = world
                            .storages()
                            .sparse_sets
                            .get(component_id)
                            .expect("Set has component");

                        set.get_with_ticks(entity.entity()).expect("Set has entity")
                    }
                };

                // SAFETY: Since we have an &World, no one should have mutable access to world
                let last_changed = unsafe { tick.changed.read() };

                let last_updated = entity_map
                    .local_modified
                    .get(&entity.entity())
                    .copied()
                    .unwrap_or(ticks.last_run());

                let last_run = if last_updated.is_newer_than(ticks.last_run(), ticks.this_run()) {
                    last_updated
                } else {
                    ticks.last_run()
                };

                let changed = last_changed.is_newer_than(last_run, ticks.this_run());
                if changed || added {
                    // SAFETY: Pointer and type adapter should match
                    let serialized = unsafe {
                        sync_info
                            .type_adapter
                            .serialize(ptr)
                            .expect("serialize error")
                    };

                    let remote_entity = entity_map
                        .local_to_forign
                        .get(&entity.entity())
                        .expect("Unmapped entity changed");

                    events.send(SerializedChangeOutEvent(
                        SerializedChange::ComponentUpdated(
                            *remote_entity,
                            sync_info.type_name.into(),
                            Some(serialized),
                        ),
                    ));
                }
            }
        }
    }
}

// Detect when components are removed
// TODO: Can this be merged with detect change?
fn detect_removals(
    settings: Res<SerializationSettings>,
    entity_map: Res<EntityMap>,
    removals: &RemovedComponentEvents,
    entities: Query<EntityRef, With<Replicate>>,
    ticks: SystemChangeTick,
    mut events: EventWriter<SerializedChangeOutEvent>,
) {
    for (component_id, sync_info) in &settings.tracked_components {
        let Some(removal_events) = removals.get(*component_id) else {
            continue;
        };

        for entity_id in removal_events.iter_current_update_events() {
            let entity_id = entity_id.clone().into();

            let Ok(entity) = entities.get(entity_id) else {
                continue;
            };

            if entity.contains_id(*component_id) {
                continue;
            }

            if entity.contains_id(sync_info.ignore_component) {
                continue;
            }

            let last_updated = entity_map
                .local_modified
                .get(&entity_id)
                .copied()
                .unwrap_or(ticks.last_run());

            if last_updated.is_newer_than(ticks.last_run(), ticks.this_run()) {
                continue;
            }

            let remote_entity = entity_map
                .local_to_forign
                .get(&entity_id)
                .expect("Unmapped entity removed component");

            events.send(SerializedChangeOutEvent(
                SerializedChange::ComponentUpdated(
                    *remote_entity,
                    sync_info.type_name.into(),
                    None,
                ),
            ));
        }
    }
}

// Detect when entities despawn
// listen for removal of sync component
fn detect_despawns(
    mut entity_map: ResMut<EntityMap>,
    mut despawns: RemovedComponents<Replicate>,
    mut events: EventWriter<SerializedChangeOutEvent>,
) {
    for entity in despawns.read() {
        let remote_entity = entity_map
            .local_to_forign
            .remove(&entity)
            .expect("Unmapped entity despawned");
        entity_map.forign_to_local.remove(&remote_entity);

        events.send(SerializedChangeOutEvent(SerializedChange::EntityDespawned(
            remote_entity,
        )));
    }
}
