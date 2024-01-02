use bevy::app::{App, Plugin, PostUpdate};
use bevy::ecs::event::{Event, EventReader};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::ecs::schedule::SystemSet;
use bevy::ecs::system::ParamSet;
use bevy::ecs::{
    archetype::ArchetypeId,
    change_detection::DetectChanges,
    component::StorageType,
    entity::Entity,
    event::EventWriter,
    ptr::UnsafeCellDeref,
    query::{Added, With},
    removal_detection::{RemovedComponentEvents, RemovedComponents},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, SystemChangeTick},
    world::{EntityRef, World},
};
use bevy::utils::HashSet;

use crate::adapters::dynamic::DynamicAdapter;
use crate::adapters::TypeAdapter;

use super::{
    EntityMap, NetId, Replicate, SerializationSettings, SerializedChange, SerializedChangeInEvent,
    SerializedChangeOutEvent,
};

// TODO(mid): Events as RPC
pub struct ChangeDetectionPlugin;

impl Plugin for ChangeDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SerializedChangeOutRawEvent>();

        app.add_systems(
            PostUpdate,
            (
                detect_new_entities,
                detect_changes.after(detect_new_entities),
                detect_removals.after(detect_changes),
                detect_despawns.after(detect_removals),
                filter_detections.after(detect_despawns),
            )
                .in_set(ChangeDetectionSet),
        );
    }
}

#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ChangeDetectionSet;

#[derive(Event, Debug)]
struct SerializedChangeOutRawEvent(pub SerializedChange);

// Detect new entities
// query for added sync component
fn detect_new_entities(
    mut cmds: Commands,
    new_entities: Query<(Entity, Option<&NetId>), Added<Replicate>>,
    mut entity_map: ResMut<EntityMap>,
    mut events: EventWriter<SerializedChangeOutRawEvent>,
) {
    for (entity_id, net_id) in &new_entities {
        let remote_entity = net_id
            .or_else(|| entity_map.local_to_forign.get(&entity_id))
            .copied()
            .unwrap_or_else(NetId::random);

        entity_map.local_to_forign.insert(entity_id, remote_entity);
        entity_map.forign_to_local.insert(remote_entity, entity_id);

        cmds.entity(entity_id).insert(remote_entity);

        events.send(SerializedChangeOutRawEvent(
            SerializedChange::EntitySpawned(remote_entity),
        ));
    }
}

// Detect when entities change
// Traverse all archetypes
// filter for the ones we care about
// check for ignore components
// if any non ignored components have changed, sync them
fn detect_changes(
    mut set: ParamSet<(
        (
            &World,
            Res<SerializationSettings>,
            Res<EntityMap>,
            Res<AppTypeRegistry>,
            SystemChangeTick,
        ),
        EventWriter<SerializedChangeOutRawEvent>,
    )>,
) {
    let mut changes = Vec::new();

    let (world, settings, entity_map, registry, ticks) = set.p0();
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
                let changed = last_changed.is_newer_than(ticks.last_run(), ticks.this_run());

                if changed || added {
                    let serialized = match &sync_info.type_adapter {
                        TypeAdapter::Serde(adapter) => unsafe { adapter.serialize(ptr) },
                        TypeAdapter::Reflect(from_ptr, _) => {
                            let reflect = unsafe { from_ptr.as_reflect(ptr) };
                            let registry = registry.read();

                            DynamicAdapter::serialize(reflect, &registry)
                        }
                    }
                    .expect("serialize error");

                    let remote_entity = entity_map
                        .local_to_forign
                        .get(&entity.entity())
                        .expect("Unmapped entity changed");

                    changes.push(SerializedChangeOutRawEvent(
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

    let mut events = set.p1();
    events.send_batch(changes);
}

// Detect when components are removed
fn detect_removals(
    mut set: ParamSet<(
        (
            Res<SerializationSettings>,
            Res<EntityMap>,
            &RemovedComponentEvents,
            Query<EntityRef, With<Replicate>>,
        ),
        EventWriter<SerializedChangeOutRawEvent>,
    )>,
) {
    let mut changes = Vec::new();

    let (settings, entity_map, removals, entities) = set.p0();
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

            let remote_entity = entity_map
                .local_to_forign
                .get(&entity_id)
                .expect("Unmapped entity removed component");

            changes.push(SerializedChangeOutRawEvent(
                SerializedChange::ComponentUpdated(
                    *remote_entity,
                    sync_info.type_name.into(),
                    None,
                ),
            ));
        }
    }

    let mut events = set.p1();
    events.send_batch(changes);
}

// Detect when entities despawn
// listen for removal of sync component
fn detect_despawns(
    mut entity_map: ResMut<EntityMap>,
    mut despawns: RemovedComponents<Replicate>,
    mut events: EventWriter<SerializedChangeOutRawEvent>,
) {
    for entity in despawns.read() {
        let Some(remote_entity) = entity_map.local_to_forign.remove(&entity) else {
            // Entity got spawned and despawned in the same change application tick?
            continue;
        };
        entity_map.forign_to_local.remove(&remote_entity);

        events.send(SerializedChangeOutRawEvent(
            SerializedChange::EntityDespawned(remote_entity),
        ));
    }
}

fn filter_detections(
    mut raw: EventReader<SerializedChangeOutRawEvent>,
    mut inbound: EventReader<SerializedChangeInEvent>,
    mut events: EventWriter<SerializedChangeOutEvent>,
) {
    let inbound = inbound.read().map(|it| &it.0).collect::<HashSet<_>>();

    events.send_batch(
        raw.read()
            .map(|it| it.0.clone())
            .filter(|it| !inbound.contains(it))
            .map(SerializedChangeOutEvent),
    );
}
