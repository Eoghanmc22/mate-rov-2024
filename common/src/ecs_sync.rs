pub mod apply_changes;
pub mod detect_changes;

use std::{any::TypeId, borrow::Cow, marker::PhantomData};

use ahash::{HashMap, HashSet};
use bevy::{
    app::App,
    ecs::{
        component::{Component, ComponentId, Tick},
        entity::Entity,
        event::Event,
        reflect::ReflectComponent,
        system::Resource,
        world::{EntityWorldMut, FromWorld, World},
    },
    reflect::{FromType, GetTypeRegistration, Reflect, ReflectFromPtr, Typed},
};
use networking::Token;
use serde::{Deserialize, Serialize};

use crate::adapters::{
    self,
    serde::{ReflectSerdeAdapter, SerdeAdapter},
    TypeAdapter,
};

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NetId(u128);

impl NetId {
    pub fn random() -> Self {
        Self(rand::random())
    }
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ForignOwned(pub(crate) usize);

pub type NetTypeId = Cow<'static, str>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SerializedChange {
    EntitySpawned(NetId),
    EntityDespawned(NetId),
    ComponentUpdated(NetId, NetTypeId, Option<adapters::BackingType>),
    EventEmitted(NetTypeId, adapters::BackingType),
}

#[derive(Event, Debug)]
pub struct SerializedChangeInEvent(pub SerializedChange, pub Token);
#[derive(Event, Debug)]
pub struct SerializedChangeOutEvent(pub SerializedChange);

#[derive(Resource, Default)]
pub struct EntityMap {
    pub(crate) local_to_forign: HashMap<Entity, NetId>,
    pub(crate) forign_to_local: HashMap<NetId, Entity>,

    pub(crate) forign_owned: HashMap<Token, HashSet<Entity>>,

    local_modified: HashMap<Entity, Tick>,
}

#[derive(Resource)]
pub struct SerializationSettings {
    marker_id: ComponentId,
    component_lookup: HashMap<NetTypeId, ComponentId>,
    tracked_components: HashMap<ComponentId, ComponentInfo>,
}

#[derive(Clone)]
pub struct ComponentInfo {
    type_name: &'static str,
    type_id: TypeId,
    component_id: ComponentId,
    type_adapter: TypeAdapter,
    ignore_component: ComponentId,
    remove_fn: RemoveFn,
}

pub type RemoveFn = fn(&mut EntityWorldMut);

#[derive(Component, Reflect)]
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
        C: Component + Typed + GetTypeRegistration + SerdeAdapter;

    fn replicate_reflect<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + Default;
}

impl AppReplicateExt for App {
    fn replicate<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + SerdeAdapter,
    {
        replicate_inner::<C>(
            self,
            TypeAdapter::Serde(<ReflectSerdeAdapter as FromType<C>>::from_type()),
        );

        self
    }

    fn replicate_reflect<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + Default,
    {
        replicate_inner::<C>(
            self,
            TypeAdapter::Reflect(
                <ReflectFromPtr as FromType<C>>::from_type(),
                <ReflectComponent as FromType<C>>::from_type(),
            ),
        );

        self
    }
}

fn replicate_inner<C>(app: &mut App, type_adapter: TypeAdapter)
where
    C: Component + Typed + GetTypeRegistration,
{
    app.register_type::<C>();

    let component_id = app.world.init_component::<C>();
    let ignored_id = app.world.init_component::<Ignore<C>>();

    let component_info = ComponentInfo {
        type_name: C::type_path(),
        type_id: TypeId::of::<C>(),
        component_id,
        type_adapter,
        ignore_component: ignored_id,
        remove_fn: |entity| {
            entity.remove::<C>();
        },
    };

    let mut settings = app.world.resource_mut::<SerializationSettings>();
    settings
        .component_lookup
        .insert(component_info.type_name.into(), component_id);
    settings
        .tracked_components
        .insert(component_id, component_info);
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
