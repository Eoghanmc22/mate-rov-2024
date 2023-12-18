//! Infrastructure to serialize and recover data

use std::{any::Any, sync::Arc};

use anyhow::Context;
use bevy::ecs::ptr::{OwningPtr, Ptr};
use bevy::reflect::{FromType, Reflect};
use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::instrument;

// TODO: Should this be Arc?
pub type BackingType = Arc<Vec<u8>>;

/// Repersents a type that can be serialized to and deserialized from another type
pub trait TypeAdapter {
    /// Serializes the provided object as [Output]
    ///
    /// # Safety
    ///
    /// Pointer must be valid and point to data of type `Self`
    unsafe fn serialize(ptr: Ptr<'_>) -> Result<BackingType, AdapterError>;
    /// Deserializes the provided output into an object
    fn deserialize(
        data: &BackingType,
        f: &mut dyn FnMut(OwningPtr<'_>),
    ) -> Result<(), AdapterError>;
}

/// Default blanket impl of TypeAdapter using the [`bincode`] trait
impl<T> TypeAdapter for T
where
    for<'a> T: Serialize + Deserialize<'a> + Any + Send + Sync,
{
    #[instrument(level = "trace", skip_all)]
    unsafe fn serialize(ptr: Ptr<'_>) -> Result<BackingType, AdapterError> {
        let val = unsafe { ptr.deref::<T>() };
        options()
            .serialize(val)
            .context("Bincode error")
            .map(Into::into)
            .map_err(AdapterError::SerializationError)
    }

    #[instrument(level = "trace", skip_all)]
    fn deserialize(
        data: &BackingType,
        f: &mut dyn FnMut(OwningPtr<'_>),
    ) -> Result<(), AdapterError> {
        let val = options()
            .deserialize::<T>(data)
            .context("Bincode error")
            .map_err(AdapterError::SerializationError)?;

        OwningPtr::make(val, f);

        Ok(())
    }
}

#[derive(Clone)]
pub struct ReflectTypeAdapter {
    serialize: unsafe fn(Ptr) -> Result<BackingType, AdapterError>,
    // TODO: Can this api be improved?
    deserialize: fn(&BackingType, &mut dyn FnMut(OwningPtr<'_>)) -> Result<(), AdapterError>,
}

impl ReflectTypeAdapter {
    pub unsafe fn serialize(&self, ptr: Ptr<'_>) -> Result<BackingType, AdapterError> {
        (self.serialize)(ptr)
    }
    pub fn deserialize<F: FnMut(OwningPtr<'_>)>(
        &self,
        data: &BackingType,
        mut handler: F,
    ) -> Result<(), AdapterError> {
        (self.deserialize)(data, &mut handler)
    }
}

impl<T> FromType<T> for ReflectTypeAdapter
where
    for<'a> T: Reflect + Serialize + Deserialize<'a> + Any + Send + Sync,
{
    fn from_type() -> Self {
        Self {
            serialize: <T as TypeAdapter>::serialize,
            deserialize: <T as TypeAdapter>::deserialize,
        }
    }
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
