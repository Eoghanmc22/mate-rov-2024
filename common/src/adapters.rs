//! Infrastructure to serialize and recover data

pub mod dynamic;
pub mod serde;

use std::sync::Arc;

use bevy::{
    ecs::{reflect::ReflectComponent, world::World},
    ptr::OwningPtr,
    reflect::ReflectFromPtr,
};
use bincode::{DefaultOptions, Options};
use thiserror::Error;

use crate::reflect::ReflectEvent;

use self::serde::ReflectSerdeAdapter;

// TODO(low): Should this be Arc?
pub type BackingType = Arc<Vec<u8>>;

#[derive(Clone)]
pub enum ComponentTypeAdapter {
    Serde(ReflectSerdeAdapter),
    Reflect(ReflectFromPtr, ReflectComponent),
}

#[derive(Clone)]
pub enum EventTypeAdapter {
    // TODO: Make a type to store this function, or come up with a better approch
    Serde(ReflectSerdeAdapter, unsafe fn(&mut World, OwningPtr<'_>)),
    Reflect(ReflectFromPtr, ReflectEvent),
}

/// The serializeation settings used
fn options() -> impl Options {
    DefaultOptions::new()
}

/// Error type used by adapters
#[derive(Error, Debug)]
pub enum AdapterError {
    /// The data could not be serialized or deserialized
    #[error("The value could not be serialized {0}")]
    SerializationError(anyhow::Error),

    /// The object passed to serialize or deserialize did not have the expected type
    #[error("Could not downcast value to a {expected_type_name}.")]
    DowncastError {
        /// The name of the type expected
        expected_type_name: &'static str,
    },

    /// No adapter matching the token could be found
    #[error("Could not find adapter for provived token")]
    NoAdapter,
}
