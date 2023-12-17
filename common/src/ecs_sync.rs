pub mod apply_changes;
pub mod detect_changes;

use std::{marker::PhantomData, sync::Arc};

use ahash::HashMap;
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
    token,
};

#[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NetId(u128);

impl NetId {
    fn random() -> Self {
        Self(rand::random())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedChange {
    EntitySpawned(NetId),
    EntityDespawned(NetId),
    ComponentUpdated(NetId, token::Key, Option<adapters::BackingType>),
    EventEmitted(token::Key, Option<adapters::BackingType>),
}

#[derive(Event, Debug)]
pub struct SerializedChangeInEvent(pub SerializedChange);
#[derive(Event, Debug)]
pub struct SerializedChangeOutEvent(pub SerializedChange);

#[derive(Resource)]
pub struct SerializationSettings {
    marker_id: ComponentId,
    component_lookup: HashMap<token::Key, ComponentId>,
    tracked_components: HashMap<ComponentId, ComponentInfo>,
}

#[derive(Resource)]
pub struct EntityMap {
    local_to_forign: HashMap<Entity, NetId>,
    forign_to_local: HashMap<NetId, Entity>,

    local_modified: HashMap<Entity, Tick>,
}

pub struct ComponentInfo {
    net_id: token::Key,
    ignore_component: ComponentId,
    adapter: Arc<dyn TypeAdapter<adapters::BackingType> + Send + Sync>,
    remove_fn: RemoveFn,
}

pub type RemoveFn = fn(&mut EntityWorldMut);

#[derive(Component)]
pub struct Replicate;
#[derive(Component)]
pub struct Ignore<T>(PhantomData<fn(T)>);

impl FromWorld for SerializationSettings {
    fn from_world(world: &mut World) -> Self {
        // let mut component_deserialization = HashMap::default();
        //
        // let adapters_components = components::adapters_components();
        // let tracked_components = adapters_components
        //     .into_iter()
        //     .map(|(key, (adapter, descriptor, remover))| {
        //         let adapter = adapter.into();
        //         let component_id = world.init_component_with_descriptor(descriptor);
        //
        //         component_deserialization
        //             .insert(key.clone(), (Arc::clone(&adapter), component_id, remover));
        //
        //         (component_id, (key, adapter))
        //     })
        //     .collect();
        //
        // let mut resource_deserialization = HashMap::default();
        //
        // let adapters_resources = components::adapters_resources();
        // let tracked_resources = adapters_resources
        //     .into_iter()
        //     .map(|(key, (adapter, type_id))| {
        //         let adapter = adapter.into();
        //
        //         resource_deserialization.insert(key.clone(), (Arc::clone(&adapter), type_id));
        //
        //         (type_id, (key, adapter))
        //     })
        //     .collect();
        //
        // SerializationSettings {
        //     tracked_components,
        //     tracked_resources,
        //     component_deserialization,
        //     resource_deserialization,
        // }

        todo!()
    }
}

// #[cfg(test)]
// mod tests {
//     use bevy_ecs::{
//         event::Events,
//         system::{IntoSystem, System},
//         world::World,
//     };
//     use tracing::Level;
//
//     use crate::components::Test;
//
//     use super::{detect_changes, SerializationSettings, SerializedChangeEventOut, SyncState};
//
//     #[test]
//     fn detect_changes() {
//         tracing_subscriber::fmt()
//             .pretty()
//             .with_max_level(Level::TRACE)
//             .init();
//
//         let mut system = IntoSystem::into_system(detect_changes::detect_changes);
//         let mut world = World::new();
//         world.init_resource::<SyncState>();
//         world.init_resource::<SerializationSettings>();
//         world.init_resource::<Events<SerializedChangeEventOut>>();
//
//         let entity = world.spawn(Test(0)).id();
//
//         system.initialize(&mut world);
//         system.run((), &mut world);
//
//         world.entity_mut(entity).insert(Test(1));
//         system.run((), &mut world);
//
//         world.entity_mut(entity).insert(Test(2));
//         world.insert_resource(Test(100));
//         system.run((), &mut world);
//
//         world.entity_mut(entity).remove::<Test>();
//         world.insert_resource(Test(101));
//         system.run((), &mut world);
//
//         world.entity_mut(entity).despawn();
//         world.remove_resource::<Test>();
//         system.run((), &mut world);
//
//         world
//             .resource_mut::<Events<SerializedChangeEventOut>>()
//             .drain()
//             .for_each(|it| println!("{it:?}"));
//
//         panic!()
//     }
// }
