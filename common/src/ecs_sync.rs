pub mod apply_changes;
pub mod detect_changes;

use std::{any::TypeId, collections::HashMap, sync::Arc};

use bevy_ecs::{
    component::{Component, ComponentId, Tick},
    entity::Entity,
    event::Event,
    system::Resource,
    world::{EntityMut, FromWorld, World},
};

use crate::{
    adapters::{self, TypeAdapter},
    components, token,
};

// TODO: Mechanism to handle newly attached peers and/or sync existing ones

#[derive(Component, Hash, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkId(pub(crate) u128);

impl NetworkId {
    pub fn random() -> Self {
        Self(rand::random())
    }
}

pub enum SerializedChange {
    EntitySpawned(NetworkId),
    EntityDespawned(NetworkId),
    ComponentUpdated(NetworkId, token::Key, Option<adapters::BackingType>),
    ResourceUpdated(token::Key, Option<adapters::BackingType>),
}

#[derive(Event)]
pub struct SerializedChangeEventIn(SerializedChange);
#[derive(Event)]
pub struct SerializedChangeEventOut(SerializedChange);

impl From<SerializedChange> for SerializedChangeEventIn {
    fn from(value: SerializedChange) -> Self {
        Self(value)
    }
}

impl From<SerializedChange> for SerializedChangeEventOut {
    fn from(value: SerializedChange) -> Self {
        Self(value)
    }
}

#[derive(Resource, Default, Debug)]
pub struct SyncState {
    components: HashMap<ComponentId, HashMap<Entity, (Semantics, Tick)>>,
    resources: HashMap<ComponentId, (Semantics, Tick)>,
}

// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// pub enum Ownership {
//     Local,
//     Forign,
// }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Semantics {
    LocalMutable,
    ForignMutable,
}

#[derive(Resource)]
pub struct SerializationSettings {
    tracked_components: HashMap<
        ComponentId,
        (
            token::Key,
            Arc<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
        ),
    >,
    tracked_resources: HashMap<
        TypeId,
        (
            token::Key,
            Arc<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
        ),
    >,

    component_deserialization: HashMap<
        token::Key,
        (
            Arc<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
            ComponentId,
            fn(&mut EntityMut),
        ),
    >,

    resource_deserialization: HashMap<
        token::Key,
        (
            Arc<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
            TypeId,
        ),
    >,
}

impl FromWorld for SerializationSettings {
    fn from_world(world: &mut World) -> Self {
        let mut component_deserialization = HashMap::new();

        let adapters_components = components::adapters_components();
        let tracked_components = adapters_components
            .into_iter()
            .map(|(key, (adapter, descriptor, remover))| {
                // TODO: Can we get rid of and Arc:: without needing to specify types?
                let id = world.init_component_with_descriptor(descriptor);
                let adapter = adapter.into();

                component_deserialization.insert(key.clone(), (Arc::clone(&adapter), id, remover));

                (id, (key, adapter))
            })
            .collect();

        let mut resource_deserialization = HashMap::new();

        let adapters_resources = components::adapters_resources();
        let tracked_resources = adapters_resources
            .into_iter()
            .map(|(key, (adapter, type_id))| {
                let adapter = adapter.into();

                resource_deserialization.insert(key.clone(), (Arc::clone(&adapter), type_id));

                (type_id, (key, adapter))
            })
            .collect();

        SerializationSettings {
            tracked_components,
            tracked_resources,
            component_deserialization,
            resource_deserialization,
        }
    }
}
