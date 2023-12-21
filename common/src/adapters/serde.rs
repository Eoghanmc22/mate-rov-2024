use anyhow::Context;
use bevy::{
    ptr::{OwningPtr, Ptr},
    reflect::{FromType, Reflect},
};
use bincode::Options;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::{options, AdapterError, BackingType};

/// Repersents a type that can be serialized to and deserialized from another type
pub trait SerdeAdapter {
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
impl<T> SerdeAdapter for T
where
    for<'a> T: Serialize + Deserialize<'a>,
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
pub struct ReflectSerdeAdapter {
    serialize: unsafe fn(Ptr) -> Result<BackingType, AdapterError>,
    // TODO: Can this api be improved?
    deserialize: fn(&BackingType, &mut dyn FnMut(OwningPtr<'_>)) -> Result<(), AdapterError>,
}

impl ReflectSerdeAdapter {
    /// Serializes the provided object as [Output]
    ///
    /// # Safety
    ///
    /// Pointer must be valid and point to data of type `Self`
    pub unsafe fn serialize(&self, ptr: Ptr<'_>) -> Result<BackingType, AdapterError> {
        (self.serialize)(ptr)
    }

    /// Deserializes the provided output into an object
    pub fn deserialize<F: FnMut(OwningPtr<'_>)>(
        &self,
        data: &BackingType,
        mut handler: F,
    ) -> Result<(), AdapterError> {
        (self.deserialize)(data, &mut handler)
    }
}

impl<T> FromType<T> for ReflectSerdeAdapter
where
    for<'a> T: Reflect + SerdeAdapter,
{
    fn from_type() -> Self {
        Self {
            serialize: <T as SerdeAdapter>::serialize,
            deserialize: <T as SerdeAdapter>::deserialize,
        }
    }
}
