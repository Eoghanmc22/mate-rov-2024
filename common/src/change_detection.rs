use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    mem,
};

use bevy_ecs::{
    archetype::ArchetypeId,
    component::{ComponentId, StorageType},
    entity::Entity,
    event::{Event, EventWriter, ManualEventReader},
    removal_detection::RemovedComponentEntity,
    storage::{TableId, TableRow},
    system::{Commands, Local, Res, Resource, SystemChangeTick},
    world::{FromWorld, World},
};

use crate::{
    adapters::{self, TypeAdapter},
    components,
    ecs_sync::NetworkId,
    token,
};

#[derive(Resource)]
pub struct ChangeDetectionSettings {
    tracked_components: HashMap<
        ComponentId,
        (
            token::Key,
            Box<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
        ),
    >,
    tracked_resources: HashMap<
        TypeId,
        (
            token::Key,
            Box<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
        ),
    >,
}

impl FromWorld for ChangeDetectionSettings {
    fn from_world(world: &mut World) -> Self {
        let adapters_components = components::adapters_components();
        let tracked_components = adapters_components
            .into_iter()
            .map(|(key, (adapter, descriptor))| {
                (
                    world.init_component_with_descriptor(descriptor),
                    (key, adapter),
                )
            })
            .collect();

        let adapters_resources = components::adapters_resources();
        let tracked_resources = adapters_resources
            .into_iter()
            .map(|(key, (adapter, type_id))| (type_id, (key, adapter)))
            .collect();

        ChangeDetectionSettings {
            tracked_components,
            tracked_resources,
        }
    }
}

// Should the last changed tick be sent?
#[derive(Event)]
pub enum SerializedChangeEvent {
    EntitySpawned(NetworkId),
    EntityDespawned(NetworkId),
    ComponentUpdated(NetworkId, token::Key, Option<adapters::BackingType>),
    ResourceUpdated(token::Key, Option<adapters::BackingType>),
}

#[derive(Default)]
struct State {
    removed_component_readers: HashMap<ComponentId, ManualEventReader<RemovedComponentEntity>>,

    // Abuses current bevy internals
    unique_archatypes: usize,
    relevant_tables: HashMap<usize, Vec<ComponentId>>,
    relevant_sets: HashSet<ComponentId>,

    cached_net_ids: HashMap<Entity, NetworkId>,

    last_resources: HashSet<TypeId>,
}

pub fn detect_changes(
    mut cmds: Commands,

    world: &World,
    mut state: Local<State>,
    settings: Res<ChangeDetectionSettings>,
    tick: SystemChangeTick,

    mut changes: EventWriter<SerializedChangeEvent>,
) {
    // Reborrow state
    let state = &mut *state;

    let mut unsynced_entities = HashSet::new();

    // FIXME: Make sure infinite loops are impossible
    // FIXME: Check that order wont cause sync issues
    // FIXME: Check the NetworkId assignment stratagy doesnt cause conflicts

    // Detect changed and added components
    let new_unique_archatypes = world.archetypes().len();
    for archetype_id in state.unique_archatypes..new_unique_archatypes {
        // Abuse bevy internals
        let archetype_id: ArchetypeId = unsafe { mem::transmute_copy(&(archetype_id as u32)) };
        let archetype = &world.archetypes()[archetype_id];
        for component_type in settings.tracked_components.keys() {
            let storage = archetype.get_storage_type(*component_type);
            match storage {
                Some(StorageType::Table) => {
                    let components = state
                        .relevant_tables
                        .entry(archetype.table_id().index())
                        .or_default();
                    components.push(*component_type);
                }
                Some(StorageType::SparseSet) => {
                    state.relevant_sets.insert(*component_type);
                }
                _ => (),
            }
        }
    }
    // This is not an intended use case
    // I need to fork bevy...
    for (table, components) in &state.relevant_tables {
        // TODO: Is this unwrap safe?
        let table = world.storages().tables.get(TableId::new(*table)).unwrap();
        for component_type in components {
            let column = table.get_column(*component_type).unwrap();
            let changed_ticks = column.get_changed_ticks_slice();
            for (idx, changed_tick) in changed_ticks.into_iter().enumerate() {
                // I love unsafe
                let last_changed = unsafe { *changed_tick.get() };
                let changed = last_changed.is_newer_than(tick.last_run(), tick.this_run());
                if changed {
                    let entity = table.entities()[idx];
                    let id = world.entity(entity).get::<NetworkId>();
                    if let Some(id) = id {
                        // Serialize the new component
                        let ptr = column.get_data(TableRow::new(idx.into())).unwrap();
                        let (token, type_adapter) =
                            settings.tracked_components.get(component_type).unwrap();
                        // Fun
                        let obj = unsafe { type_adapter.deref(ptr) };
                        // TODO: error handeling
                        let serialized = type_adapter.serialize(obj).expect("serialize error");

                        changes.send(SerializedChangeEvent::ComponentUpdated(
                            *id,
                            token.clone(),
                            Some(serialized),
                        ));
                    } else {
                        unsynced_entities.insert(entity);
                    }
                }
            }
        }
    }
    for _component in &state.relevant_sets {
        // This literally doesnt seem to be possible with the exposed api...
        panic!("`SparseSet`s are not supported");
    }

    // Detect removed components
    for (component_type, (token, _)) in &settings.tracked_components {
        let events = world.removed_components().get(*component_type);
        if let Some(events) = events {
            let reader = state
                .removed_component_readers
                .entry(*component_type)
                .or_insert_with(|| events.get_reader());
            for event in reader.iter(events) {
                let entity_id = event.clone().into();
                let entity = world.get_entity(entity_id);
                if let Some(entity) = entity {
                    let id = entity.get::<NetworkId>();
                    if let Some(id) = id {
                        changes.send(SerializedChangeEvent::ComponentUpdated(
                            *id,
                            token.clone(),
                            None,
                        ));
                    } else {
                        unsynced_entities.insert(entity_id);
                    }
                } else {
                    // Entity was despawned
                    // See if we remember its NetworkId
                    let network_id = state.cached_net_ids.remove(&entity_id);
                    if let Some(network_id) = network_id {
                        changes.send(SerializedChangeEvent::EntityDespawned(network_id));
                    } else {
                        // This gets hit for every component the entity used to have
                    }
                }
            }
        }
    }

    // Detect changed and added resources
    let mut resources = HashSet::new();
    for (type_id, (token, type_adapter)) in &settings.tracked_resources {
        let resource = world
            .components()
            .get_resource_id(*type_id)
            .and_then(|component_id| world.storages().resources.get(component_id));

        if let Some(resource) = resource {
            if let (Some(ptr), Some(ticks)) = (resource.get_data(), resource.get_ticks()) {
                resources.insert(*type_id);

                let changed = ticks
                    .last_changed_tick()
                    .is_newer_than(tick.last_run(), tick.this_run());
                if changed {
                    // Serialize the new resource
                    // Fun
                    let obj = unsafe { type_adapter.deref(ptr) };
                    // TODO: error handeling
                    let serialized = type_adapter.serialize(obj).expect("serialize error");

                    changes.send(SerializedChangeEvent::ResourceUpdated(
                        token.clone(),
                        Some(serialized),
                    ));
                }
            }
        }
    }
    //
    // Detect removed resources
    let deleted = state.last_resources.difference(&resources);
    for resource in deleted {
        let (token, _) = settings.tracked_resources.get(resource).unwrap();

        changes.send(SerializedChangeEvent::ResourceUpdated(token.clone(), None));
    }

    // Sync entities in `unsynced_entities`
    for entity in unsynced_entities {
        // assign net id
        // sync components to peer
        // add entityid netid pair to map in state

        let net_id = NetworkId::random();
        cmds.entity(entity).insert(net_id);
        changes.send(SerializedChangeEvent::EntitySpawned(net_id));
        let component_types = world.inspect_entity(entity);
        for component_type in component_types {
            // Serialize the new component
            let ptr = world.entity(entity).get_by_id(component_type.id()).unwrap();
            let (token, type_adapter) = settings
                .tracked_components
                .get(&component_type.id())
                .unwrap();
            // Fun
            let obj = unsafe { type_adapter.deref(ptr) };
            // TODO: error handeling
            let serialized = type_adapter.serialize(obj).expect("serialize error");

            changes.send(SerializedChangeEvent::ComponentUpdated(
                net_id,
                token.clone(),
                Some(serialized),
            ));
        }
        state.cached_net_ids.insert(entity, net_id);
    }
}
