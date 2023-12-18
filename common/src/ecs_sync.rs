pub mod apply_changes;
pub mod detect_changes;

use std::{any::TypeId, borrow::Cow, marker::PhantomData};

use ahash::HashMap;
use bevy::{
    app::App,
    ecs::{
        component::{Component, ComponentId, Tick},
        entity::Entity,
        event::Event,
        system::Resource,
        world::{EntityWorldMut, FromWorld, World},
    },
    reflect::{FromType, GetTypeRegistration, Reflect, Typed},
};
use serde::{Deserialize, Serialize};

use crate::adapters::{self, ReflectTypeAdapter, TypeAdapter};

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NetId(u128);

impl NetId {
    fn random() -> Self {
        Self(rand::random())
    }
}

pub type NetTypeId = Cow<'static, str>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedChange {
    EntitySpawned(NetId),
    EntityDespawned(NetId),
    ComponentUpdated(NetId, NetTypeId, Option<adapters::BackingType>),
    EventEmitted(NetTypeId, adapters::BackingType),
}

#[derive(Event, Debug)]
pub struct SerializedChangeInEvent(pub SerializedChange);
#[derive(Event, Debug)]
pub struct SerializedChangeOutEvent(pub SerializedChange);

#[derive(Resource, Default)]
pub struct EntityMap {
    local_to_forign: HashMap<Entity, NetId>,
    forign_to_local: HashMap<NetId, Entity>,

    local_modified: HashMap<Entity, Tick>,
}

#[derive(Resource)]
pub struct SerializationSettings {
    marker_id: ComponentId,
    component_lookup: HashMap<NetTypeId, ComponentId>,
    tracked_components: HashMap<ComponentId, ComponentInfo>,
}

pub struct ComponentInfo {
    type_name: &'static str,
    type_id: TypeId,
    component_id: ComponentId,
    type_adapter: ReflectTypeAdapter,
    ignore_component: ComponentId,
    remove_fn: RemoveFn,
}

pub type RemoveFn = fn(&mut EntityWorldMut);

#[derive(Component)]
pub struct Replicate;
#[derive(Component)]
pub struct Ignore<T>(PhantomData<fn(T)>);

impl FromWorld for SerializationSettings {
    fn from_world(world: &mut World) -> Self {
        let marker_id = world.init_component::<Replicate>();

        Self {
            marker_id,
            component_lookup: Default::default(),
            tracked_components: Default::default(),
        }
    }
}

pub trait AppReplicateExt {
    fn replicate<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + TypeAdapter;
}

impl AppReplicateExt for App {
    fn replicate<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + TypeAdapter,
    {
        self.register_type::<C>();

        let component_id = self.world.init_component::<C>();
        let ignored_id = self.world.init_component::<Ignore<C>>();

        let component_info = ComponentInfo {
            type_name: C::type_path(),
            type_id: TypeId::of::<C>(),
            component_id,
            type_adapter: <ReflectTypeAdapter as FromType<C>>::from_type(),
            ignore_component: ignored_id,
            remove_fn: |entity| {
                entity.remove::<C>();
            },
        };

        let mut settings = self.world.resource_mut::<SerializationSettings>();
        settings
            .component_lookup
            .insert(component_info.type_name.into(), component_id);
        settings
            .tracked_components
            .insert(component_id, component_info);

        self
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
