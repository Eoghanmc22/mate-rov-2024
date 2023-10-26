pub mod apply_changes;
pub mod detect_changes;

use std::collections::HashMap;

use bevy_ecs::{
    component::{Component, ComponentId, Tick},
    entity::Entity,
    event::Event,
    system::Resource,
};

use crate::{adapters, token};

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
