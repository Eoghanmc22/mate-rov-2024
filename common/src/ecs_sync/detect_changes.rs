use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    mem,
};

use bevy_ecs::{
    archetype::ArchetypeId,
    component::{ComponentId, StorageType, Tick},
    entity::Entity,
    event::ManualEventReader,
    ptr::Ptr,
    removal_detection::RemovedComponentEntity,
    storage::{TableId, TableRow},
    system::{CommandQueue, Commands, Local, SystemChangeTick, SystemState},
    world::World,
};
use tracing::error;

use crate::{
    adapters::{self, BackingType, TypeAdapter},
    ecs_sync::NetworkId,
};

use super::{
    Semantics, SerializationSettings, SerializedChange, SerializedChangeEventOut, SyncState,
};

#[derive(Default)]
pub struct ChangeDetectionState {
    removed_component_readers: HashMap<ComponentId, ManualEventReader<RemovedComponentEntity>>,

    // Abuses current bevy internals
    unique_archetypes: usize,
    relevant_tables: HashMap<usize, Vec<ComponentId>>,
    relevant_sets: HashSet<ComponentId>,

    cached_local_net_ids: HashMap<Entity, NetworkId>,

    last_resources: HashSet<TypeId>,
}

pub fn detect_changes(
    world: &mut World,
    mut state: Local<ChangeDetectionState>,
    tick: &mut SystemState<SystemChangeTick>,
) {
    // Reborrows
    let mut sync_state = world.remove_resource::<SyncState>().unwrap();
    let settings = world.get_resource::<SerializationSettings>().unwrap();
    let state = &mut *state;

    let mut new_entities = HashSet::new();
    let mut changes = Vec::new();

    let mut queue = CommandQueue::default();
    let mut cmds = Commands::new(&mut queue, world);

    let tick = tick.get(world);

    // FIXME: Make sure infinite loops are impossible
    // FIXME: Make sure we dont miss changes
    // FIXME: Check that order wont cause sync issues
    // TODO: Validate that state is matained consistently

    // Handle changes to entities
    filter_new_archetypes(world, settings, state);
    detect_changes_tables(
        world,
        settings,
        state,
        &mut sync_state,
        &tick,
        &mut changes,
        &mut new_entities,
    );
    detect_changes_sparse_set(
        world,
        settings,
        state,
        &mut sync_state,
        &tick,
        &mut changes,
        &mut new_entities,
    );
    detect_removed_components(world, settings, state, &mut sync_state, &tick, &mut changes);
    sync_new_entities(
        &mut cmds,
        world,
        settings,
        state,
        &mut sync_state,
        &mut changes,
        &new_entities,
    );

    // Handle changes to resources
    handle_changed_resources(world, settings, state, &mut sync_state, &tick, &mut changes);

    world.insert_resource(sync_state);
    world.send_event_batch(changes);
    queue.apply(world);
}

/// Checks each new archetype since last system run for matches with tracked components
fn filter_new_archetypes(
    world: &World,
    settings: &SerializationSettings,
    state: &mut ChangeDetectionState,
) {
    // Abuse bevy internals
    // In bevy, archetypes are only added
    // Iterate over the new indices
    let new_unique_archetypes = world.archetypes().len();

    for archetype_id in state.unique_archetypes..new_unique_archetypes {
        // Theres no public constructor for ArchetypeId...
        let archetype_id: ArchetypeId = unsafe { mem::transmute_copy(&(archetype_id as u32)) };
        let archetype = &world.archetypes()[archetype_id];

        println!(
            "new archetype, {:?}",
            archetype.table_components().collect::<Vec<_>>()
        );

        // Check if this archetype contains any component types we track
        for component_type in settings.tracked_components.keys() {
            println!("component in archetype, {component_type:?}");
            let Some(component_id) = world.components().get_id(*component_type) else {
                // No components of this type exist...
                continue;
            };

            let storage = archetype.get_storage_type(component_id);
            let Some(storage) = storage else {
                println!("dont care");
                // Archetype does not contain this component type
                // Check the next component type
                continue;
            };

            // Record where these components are stored so we
            // can traverse them during change detection
            match storage {
                StorageType::Table => {
                    state
                        .relevant_tables
                        .entry(archetype.table_id().index())
                        .or_default()
                        .push(component_id);
                }
                StorageType::SparseSet => {
                    state.relevant_sets.insert(component_id);
                }
            }
        }
    }

    state.unique_archetypes = new_unique_archetypes;
}

/// Detect changes to components stored in "tables"
fn detect_changes_tables(
    world: &World,
    settings: &SerializationSettings,
    state: &mut ChangeDetectionState,
    sync_state: &mut SyncState,
    tick: &SystemChangeTick,
    changes: &mut Vec<SerializedChangeEventOut>,

    new_entities: &mut HashSet<Entity>,
) {
    println!("hit");
    // This is not an intended use case
    // I need to fork bevy...

    // Check each table we recorded as containing a component type we track
    for (table, components) in &state.relevant_tables {
        println!("table");
        // Lookup the table in the ECS
        let table = world.storages().tables.get(TableId::new(*table)).unwrap();

        // Check each table column that contains a tracked component
        for component_id in components {
            println!("component type");

            let Some(component_type) = world
                .components()
                .get_info(*component_id)
                .and_then(|it| it.type_id())
            else {
                // TODO: If this is impossible use unwrap instead
                error!("BUG?");
                continue;
            };

            // Lookup the column in the table
            let column = table.get_column(*component_id).unwrap();
            let changed_ticks = column.get_changed_ticks_slice();

            // Lookup the sync metadata for this component type
            let component_sync_state = sync_state.components.entry(*component_id).or_default();

            for (idx, changed_tick) in changed_ticks.into_iter().enumerate() {
                println!("entity");
                // I love unsafe
                let last_changed = unsafe { *changed_tick.get() };

                // Determine if this change has already been seen
                let seen = last_changed.is_newer_than(tick.last_run(), tick.this_run());
                if !seen {
                    println!("seen");
                    continue;
                }

                // Determine which entity this component belongs to
                // and lookup its sync metadata for this component
                let entity_id = table.entities()[idx];
                let Some(net_id) = world.get::<NetworkId>(entity_id) else {
                    println!("new");
                    // This entity has not been seen before
                    new_entities.insert(entity_id);

                    continue;
                };
                let (semantics, last_sync_tick) = *component_sync_state
                    .entry(entity_id)
                    .or_insert_with(|| (Semantics::LocalMutable, Tick::new(0)));

                // Determine if this change was due to applying a remote change
                let modified_locally = last_changed.is_newer_than(last_sync_tick, tick.this_run());
                if !modified_locally {
                    println!("forign modified");
                    continue;
                }

                // Serialize the new component
                let ptr = column.get_data(TableRow::new(idx.into())).unwrap();
                let (token, type_adapter) =
                    settings.tracked_components.get(&component_type).unwrap();
                // SAFETY: `type_adapter` is assoicated with the component_type of this column and
                // therefore should match the type of ptr
                let serialized = unsafe { serialize_ptr(ptr, &**type_adapter) };

                // Check that this write is allowed
                if semantics != Semantics::LocalMutable {
                    error!("Local modified forign controlled component");
                }

                println!("emit");
                // Notify other systems about this change
                changes.push(
                    SerializedChange::ComponentUpdated(*net_id, token.clone(), Some(serialized))
                        .into(),
                );
            }
        }
    }
}

/// Detect changes to components stored in sparse sets
fn detect_changes_sparse_set(
    _world: &World,
    _settings: &SerializationSettings,
    state: &mut ChangeDetectionState,
    _sync_state: &mut SyncState,
    _tick: &SystemChangeTick,
    _changes: &mut Vec<SerializedChangeEventOut>,

    _new_entities: &mut HashSet<Entity>,
) {
    for _component in &state.relevant_sets {
        // This literally doesnt seem to be possible with the exposed api...
        panic!("`SparseSet`s are not supported");
    }
}

/// Detects removed components and deleted entities
fn detect_removed_components(
    world: &World,
    settings: &SerializationSettings,
    state: &mut ChangeDetectionState,
    sync_state: &mut SyncState,
    tick: &SystemChangeTick,
    changes: &mut Vec<SerializedChangeEventOut>,
) {
    // Detect despawned entitied
    for entity_id in world.removed::<NetworkId>() {
        // Entity needs to be despawned on peer
        // If entity_id is in `cached_net_ids`, it it locally owned
        if let Some(net_id) = state.cached_local_net_ids.remove(&entity_id) {
            // Sync change with peers
            changes.push(SerializedChange::EntityDespawned(net_id).into());
        } else {
            // Dont sync illegally despawned entities
            // Forign could still need them
            error!("Local deleted forign owned entity");
        }
        // Cleanup state
        for map in sync_state.components.values_mut() {
            map.remove(&entity_id);
        }
    }

    // Check each component type we track
    for (component_type, (token, _)) in &settings.tracked_components {
        let Some(component_id) = world.components().get_id(*component_type) else {
            // No components of this type exist...
            continue;
        };

        // Get the removed component event buffer
        let Some(events) = world.removed_components().get(component_id) else {
            // No components of this type have been removed yet
            continue;
        };

        // Get the event reader for this component type
        let reader = state
            .removed_component_readers
            .entry(component_id)
            .or_insert_with(|| events.get_reader());

        // Lookup the sync metadata for this component type
        let component_sync_state = sync_state.components.entry(component_id).or_default();

        // Read new events
        for event in reader.iter(events) {
            // Determine which entity this component belongs to
            let entity_id = event.clone().into();

            // Lookup network id
            // Otherwise, handle entity despawn
            let Some(net_id) = world.get::<NetworkId>(entity_id) else {
                // Entity isnt real?

                continue;
            };

            // Lookup this components semantics
            let (semantics, last_sync_tick) = *component_sync_state
                .entry(entity_id)
                .or_insert_with(|| (Semantics::LocalMutable, Tick::new(0)));

            if semantics == Semantics::LocalMutable {
                changes
                    .push(SerializedChange::ComponentUpdated(*net_id, token.clone(), None).into());
            } else {
                // Determine if this change was due to applying a remote change
                let modified_locally =
                    last_sync_tick.is_newer_than(tick.last_run(), tick.this_run());
                if modified_locally {
                    // Dont sync illegally deleted components
                    // Forign could still need them
                    error!("Local deleted forign controlled component");
                } else {
                    // Either the result of a sync or a race contidion...
                }
            }
        }
    }
}

/// Detect changed and removed resources
fn handle_changed_resources(
    world: &World,
    settings: &SerializationSettings,
    state: &mut ChangeDetectionState,
    sync_state: &mut SyncState,
    tick: &SystemChangeTick,
    changes: &mut Vec<SerializedChangeEventOut>,
) {
    // Detect changed resources
    let mut resources = HashSet::new();
    for (type_id, (token, type_adapter)) in &settings.tracked_resources {
        // Type annotations rlly ugly :(
        let _: Option<()> = try {
            let component_id = world.components().get_resource_id(*type_id)?;
            let resource = world.storages().resources.get(component_id)?;
            let ticks = resource.get_ticks()?;

            let (semantics, last_sync_tick) = *sync_state
                .resources
                .entry(component_id)
                .or_insert_with(|| (Semantics::LocalMutable, Tick::new(0)));

            // Record the presence of this resource for deletion detection
            resources.insert(*type_id);

            let changed = ticks
                .last_changed_tick()
                .is_newer_than(last_sync_tick, tick.this_run());
            if changed {
                // Serialize the new resource
                let ptr = resource.get_data()?;
                let serialized = unsafe { serialize_ptr(ptr, &**type_adapter) };

                if semantics != Semantics::LocalMutable {
                    error!("Local modified forign controlled component");
                }

                changes.push(
                    SerializedChange::ResourceUpdated(token.clone(), Some(serialized)).into(),
                );
            }
        };
    }

    // Detect deleted resources
    let deleted = state.last_resources.difference(&resources);
    for resource in deleted {
        let (token, _) = settings.tracked_resources.get(resource).unwrap();

        changes.push(SerializedChange::ResourceUpdated(token.clone(), None).into());
    }

    state.last_resources = resources;
}

fn sync_new_entities(
    cmds: &mut Commands,

    world: &World,
    settings: &SerializationSettings,
    state: &mut ChangeDetectionState,
    sync_state: &mut SyncState,

    changes: &mut Vec<SerializedChangeEventOut>,
    new_entities: &HashSet<Entity>,
) {
    for entity in new_entities {
        // Assign random network id
        let net_id = NetworkId::random();
        cmds.entity(*entity).insert(net_id);
        state.cached_local_net_ids.insert(*entity, net_id);

        // Spawn entity on peer
        changes.push(SerializedChange::EntitySpawned(net_id).into());

        // Sync components with peer
        let component_types = world.inspect_entity(*entity);
        for component_info in component_types {
            let last_sync_meta = sync_state
                .components
                .entry(component_info.id())
                .or_default()
                .insert(*entity, (Semantics::LocalMutable, Tick::new(0)));
            if let Some((semantics, tick)) = last_sync_meta {
                error!("BUG: New component is already tracked! semantics: {semantics:?}, last synced: {tick:?}");
            }

            let Some(component_type) = component_info.type_id() else {
                // TODO: If this is impossible use unwrap instead
                error!("BUG?");
                continue;
            };

            // Serialize the new component
            let ptr = world
                .entity(*entity)
                .get_by_id(component_info.id())
                .unwrap();
            let (token, type_adapter) = settings.tracked_components.get(&component_type).unwrap();
            // SAFETY: `type_adapter` is assoicated with the component_type for this component
            // therefore should match the type of ptr
            let serialized = unsafe { serialize_ptr(ptr, &**type_adapter) };

            changes.push(
                SerializedChange::ComponentUpdated(net_id, token.clone(), Some(serialized)).into(),
            );
        }
    }
}

unsafe fn serialize_ptr(
    ptr: Ptr<'_>,
    type_adapter: &(dyn TypeAdapter<adapters::BackingType> + Send + Sync),
) -> BackingType {
    // TODO: error handling
    // SAFETY: Caller is required to make sure the pointer and type_adapter match
    type_adapter.serialize(ptr).expect("serialize error")
}
