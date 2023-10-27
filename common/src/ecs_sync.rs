pub mod apply_changes;
pub mod detect_changes;

use std::{any::TypeId, sync::Arc};

use ahash::AHashMap as HashMap;
use bevy_ecs::{
    component::{Component, ComponentId, Tick},
    entity::Entity,
    event::Event,
    system::Resource,
    world::{EntityWorldMut, FromWorld, World},
};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::{self, TypeAdapter},
    components, token,
};

// TODO: Mechanism to handle newly attached peers and/or sync existing ones

#[derive(Component, Hash, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct NetworkId(pub(crate) u128);

impl NetworkId {
    pub const SINGLETON: NetworkId = NetworkId(0);

    pub fn random() -> Self {
        Self(rand::random())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedChange {
    EntitySpawned(NetworkId),
    EntityDespawned(NetworkId),
    ComponentUpdated(NetworkId, token::Key, Option<adapters::BackingType>),
    ResourceUpdated(token::Key, Option<adapters::BackingType>),
}

#[derive(Event, Debug)]
pub struct SerializedChangeEventIn(pub SerializedChange, pub usize);
#[derive(Event, Debug)]
pub struct SerializedChangeEventOut(pub SerializedChange);

impl From<SerializedChange> for SerializedChangeEventOut {
    fn from(value: SerializedChange) -> Self {
        Self(value)
    }
}

#[derive(Resource, Default, Debug)]
pub struct SyncState {
    components: HashMap<ComponentId, HashMap<Entity, (Semantics, Tick)>>,
    resources: HashMap<ComponentId, (Semantics, Tick)>,

    pub singleton_map: HashMap<usize, Entity>,
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
            fn(&mut EntityWorldMut),
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
                let adapter = adapter.into();
                let component_id = world.init_component_with_descriptor(descriptor);

                component_deserialization
                    .insert(key.clone(), (Arc::clone(&adapter), component_id, remover));

                (component_id, (key, adapter))
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

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        event::Events,
        system::{IntoSystem, System},
        world::World,
    };
    use tracing::Level;

    use crate::components::Test;

    use super::{detect_changes, SerializationSettings, SerializedChangeEventOut, SyncState};

    #[test]
    fn detect_changes() {
        tracing_subscriber::fmt()
            .pretty()
            .with_max_level(Level::TRACE)
            .init();

        let mut system = IntoSystem::into_system(detect_changes::detect_changes);
        let mut world = World::new();
        world.init_resource::<SyncState>();
        world.init_resource::<SerializationSettings>();
        world.init_resource::<Events<SerializedChangeEventOut>>();

        let entity = world.spawn(Test(0)).id();

        system.initialize(&mut world);
        system.run((), &mut world);

        world.entity_mut(entity).insert(Test(1));
        system.run((), &mut world);

        world.entity_mut(entity).insert(Test(2));
        world.insert_resource(Test(100));
        system.run((), &mut world);

        world.entity_mut(entity).remove::<Test>();
        world.insert_resource(Test(101));
        system.run((), &mut world);

        world.entity_mut(entity).despawn();
        world.remove_resource::<Test>();
        system.run((), &mut world);

        world
            .resource_mut::<Events<SerializedChangeEventOut>>()
            .drain()
            .for_each(|it| println!("{it:?}"));

        panic!()
    }
}
