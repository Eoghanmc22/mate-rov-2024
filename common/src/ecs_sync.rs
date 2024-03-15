pub mod apply_changes;
pub mod detect_changes;

use std::any::Any;
use std::sync::Arc;
use std::{any::TypeId, borrow::Cow, marker::PhantomData};

use ahash::{HashMap, HashSet};
use bevy::{
    app::App,
    ecs::{
        component::{Component, ComponentId, Tick},
        entity::Entity,
        event::{Event, Events, ManualEventReader},
        reflect::ReflectComponent,
        system::Resource,
        world::{EntityWorldMut, FromWorld, World},
    },
    ptr::Ptr,
    reflect::{FromReflect, FromType, GetTypeRegistration, Reflect, ReflectFromPtr, Typed},
};
use networking::Token;
use serde::{Deserialize, Serialize};

use crate::{
    adapters::{
        self,
        serde::{ReflectSerdeAdapter, SerdeAdapter},
        ComponentTypeAdapter, EventTypeAdapter,
    },
    reflect::ReflectEvent,
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

    pub(crate) local_modified: HashMap<Entity, Tick>,
}

#[derive(Resource)]
pub struct SerializationSettings {
    marker_id: ComponentId,

    // TODO: Store an Arc<ComponentInfo> referenced by both maps
    component_by_token: HashMap<NetTypeId, Arc<ComponentInfo>>,
    component_by_id: HashMap<ComponentId, Arc<ComponentInfo>>,

    // TODO: Store an Arc<EventInfo> referenced by both maps
    event_by_token: HashMap<NetTypeId, Arc<EventInfo>>,
    event_by_id: HashMap<ComponentId, Arc<EventInfo>>,
}

#[derive(Clone)]
pub struct ComponentInfo {
    type_name: &'static str,
    type_id: TypeId,
    component_id: ComponentId,
    type_adapter: ComponentTypeAdapter,
    ignore_component: ComponentId,
    remove_fn: RemoveFn,
}

#[derive(Clone)]
pub struct EventInfo {
    type_name: &'static str,
    type_id: TypeId,
    component_id: ComponentId,
    type_adapter: EventTypeAdapter,
    reader_factory: fn() -> ErasedManualEventReader,
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
            component_by_token: Default::default(),
            component_by_id: Default::default(),
            event_by_token: Default::default(),
            event_by_id: Default::default(),
        }
    }
}

pub trait AppReplicateExt {
    fn replicate<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + SerdeAdapter;

    fn replicate_reflect<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + FromReflect;

    fn replicate_event<C>(&mut self) -> &mut Self
    where
        C: Event + Typed + GetTypeRegistration + SerdeAdapter;

    fn replicate_event_reflect<C>(&mut self) -> &mut Self
    where
        C: Event + Typed + GetTypeRegistration + FromReflect;
}

impl AppReplicateExt for App {
    fn replicate<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + SerdeAdapter,
    {
        replicate_inner::<C>(
            self,
            ComponentTypeAdapter::Serde(<ReflectSerdeAdapter as FromType<C>>::from_type()),
        );

        self
    }

    fn replicate_reflect<C>(&mut self) -> &mut Self
    where
        C: Component + Typed + GetTypeRegistration + FromReflect,
    {
        replicate_inner::<C>(
            self,
            ComponentTypeAdapter::Reflect(
                <ReflectFromPtr as FromType<C>>::from_type(),
                <ReflectComponent as FromType<C>>::from_type(),
            ),
        );

        self
    }

    fn replicate_event<E>(&mut self) -> &mut Self
    where
        E: Event + Typed + GetTypeRegistration + SerdeAdapter,
    {
        replicate_event_inner::<E>(
            self,
            EventTypeAdapter::Serde(
                <ReflectSerdeAdapter as FromType<E>>::from_type(),
                |world, ptr| unsafe {
                    world.send_event(ptr.read::<E>());
                },
            ),
        );

        self
    }

    fn replicate_event_reflect<E>(&mut self) -> &mut Self
    where
        E: Event + Typed + GetTypeRegistration + FromReflect,
    {
        replicate_event_inner::<E>(
            self,
            EventTypeAdapter::Reflect(
                <ReflectFromPtr as FromType<E>>::from_type(),
                <ReflectEvent as FromType<E>>::from_type(),
            ),
        );

        self
    }
}

fn replicate_inner<C>(app: &mut App, type_adapter: ComponentTypeAdapter)
where
    C: Component + Typed + GetTypeRegistration,
{
    app.register_type::<C>();

    let component_id = app.world.init_component::<C>();
    let ignored_id = app.world.init_component::<Ignore<C>>();

    let component_info = Arc::new(ComponentInfo {
        type_name: C::type_path(),
        type_id: TypeId::of::<C>(),
        component_id,
        type_adapter,
        ignore_component: ignored_id,
        remove_fn: |entity| {
            entity.remove::<C>();
        },
    });

    let mut settings = app.world.resource_mut::<SerializationSettings>();
    settings
        .component_by_token
        .insert(component_info.type_name.into(), component_info.clone());
    settings
        .component_by_id
        .insert(component_id, component_info);
}

fn replicate_event_inner<E>(app: &mut App, type_adapter: EventTypeAdapter)
where
    E: Event + Typed + GetTypeRegistration,
{
    app.register_type::<E>();
    app.add_event::<E>();

    let component_id = app.world.init_resource::<Events<E>>();
    let event_info = Arc::new(EventInfo {
        type_name: E::type_path(),
        type_id: TypeId::of::<E>(),
        component_id,
        type_adapter,
        reader_factory: ErasedManualEventReader::new::<E>,
    });

    let mut settings = app.world.resource_mut::<SerializationSettings>();
    settings
        .event_by_token
        .insert(event_info.type_name.into(), event_info.clone());
    settings.event_by_id.insert(component_id, event_info);
}

pub struct ErasedManualEventReader {
    reader: Option<Box<dyn Any + Send + Sync>>,
    read_event:
        for<'a> fn(&'a World, &'a mut Option<Box<dyn Any + Send + Sync>>) -> Option<Ptr<'a>>,
}

impl ErasedManualEventReader {
    pub fn new<E: Event>() -> Self {
        ErasedManualEventReader {
            reader: None,

            read_event: |world, reader| {
                let events = world.get_resource::<Events<E>>()?;
                let reader = reader
                    .get_or_insert_with(|| Box::new(events.get_reader()))
                    .downcast_mut::<ManualEventReader<E>>()
                    .unwrap();

                reader.read(events).next().map(Into::into)
            },
        }
    }

    pub fn read_event<'a>(&'a mut self, world: &'a World) -> Option<Ptr<'a>> {
        (self.read_event)(world, &mut self.reader)
    }
}

impl Clone for ErasedManualEventReader {
    fn clone(&self) -> Self {
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
