use anyhow::Context;
use bevy::reflect::{
    serde::{TypedReflectDeserializer, TypedReflectSerializer},
    Reflect, TypeRegistration, TypeRegistry,
};
use bincode::Options;
use tracing::instrument;

use super::{options, AdapterError, BackingType};

/// Repersents a type that can be serialized to and deserialized using reflection
pub struct DynamicAdapter;

/// Default blanket impl of TypeAdapter using the [`bincode`] trait
impl DynamicAdapter {
    /// Serializes the provided object as [Output]
    #[instrument(level = "trace", skip_all)]
    pub fn serialize(
        obj: &dyn Reflect,
        registry: &TypeRegistry,
    ) -> Result<BackingType, AdapterError> {
        let val = TypedReflectSerializer::new(obj, registry);

        options()
            .serialize(&val)
            .context("Bincode error")
            .map(Into::into)
            .map_err(AdapterError::SerializationError)
    }

    /// Deserializes the provided output into an object
    #[instrument(level = "trace", skip_all)]
    pub fn deserialize(
        data: &BackingType,
        registration: &TypeRegistration,
        registry: &TypeRegistry,
    ) -> Result<Box<dyn Reflect>, AdapterError> {
        let seed = TypedReflectDeserializer::new(registration, registry);

        let val = options()
            .deserialize_seed(seed, data)
            .context("Bincode error")
            .map_err(AdapterError::SerializationError)?;

        Ok(val)
    }
}
