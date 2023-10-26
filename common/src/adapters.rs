//! Infrastructure to serialize and recover data

use std::{any::Any, marker::PhantomData};

use anyhow::Context;
use bevy_ecs::ptr::{OwningPtr, Ptr};
use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Repersents a type that can be serialized to and deserialized from another type
pub trait TypeAdapter<Output> {
    /// Serializes the provided object as [Output]
    unsafe fn serialize(&self, ptr: Ptr<'_>) -> Result<Output, AdapterError>;
    /// Deserializes the provided output into an object
    fn deserialize(
        &self,
        data: &Output,
        f: &mut dyn FnMut(OwningPtr<'_>),
    ) -> Result<(), AdapterError>;
}

/// The type aganist which TypeAdapter is implemented
#[derive(Clone, Copy)]
pub struct Adapter<T>(PhantomData<T>);

impl<B> Default for Adapter<B> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Default blanket impl of TypeAdapter using the [`bincode`] trait
pub type BackingType = Vec<u8>;
impl<T> TypeAdapter<BackingType> for Adapter<T>
where
    for<'a> T: Serialize + Deserialize<'a> + Any + Send + Sync,
{
    unsafe fn serialize(&self, ptr: Ptr<'_>) -> Result<BackingType, AdapterError> {
        let val = unsafe { ptr.deref::<T>() };
        options()
            .serialize(val)
            .context("Bincode error")
            .map_err(AdapterError::SerializationError)
    }

    fn deserialize(
        &self,
        data: &BackingType,
        f: &mut dyn FnMut(OwningPtr<'_>),
    ) -> Result<(), AdapterError> {
        let val = options()
            .deserialize::<T>(data)
            .context("Bincode error")
            .map_err(AdapterError::SerializationError)?;

        OwningPtr::make(val, |ptr| (f)(ptr));

        Ok(())
    }
}

/// The serializeation settings used
fn options() -> impl Options {
    DefaultOptions::new()
}

/// Macros to generate adapter lookup tables
#[macro_export]
macro_rules! generate_adapters_components {
    (name = $name:ident, output = $output:ty, tokens = { $($token:expr),* }) => {
        pub fn $name() -> std::collections::HashMap<$crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, bevy_ecs::component::ComponentDescriptor, fn(&mut bevy_ecs::world::EntityWorldMut))>
        {
            fn from<T: bevy_ecs::component::Component, O>(token: $crate::token::Token<T, O>) -> ($crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, bevy_ecs::component::ComponentDescriptor, fn(&mut bevy_ecs::world::EntityWorldMut)))
            where
                for<'a> T: Send + Sync + serde::Serialize + serde::Deserialize<'a> + 'static,
                $crate::adapters::Adapter<T>: $crate::adapters::TypeAdapter<$output>
            {
                (token.0, (std::boxed::Box::<$crate::adapters::Adapter<T>>::default(), bevy_ecs::component::ComponentDescriptor::new::<T>(), |entity| {
                    entity.remove::<T>();
                }))
            }

            vec![
                $(
                    from($token),
                )*
            ]
            .into_iter()
            .collect()
        }
    };
}
#[macro_export]
macro_rules! generate_adapters_resources {
    (name = $name:ident, output = $output:ty, tokens = { $($token:expr),* }) => {
        pub fn $name() -> std::collections::HashMap<$crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, std::any::TypeId)>
        {
            fn from<T: bevy_ecs::system::Resource, O>(token: $crate::token::Token<T, O>) -> ($crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, std::any::TypeId))
            where
                for<'a> T: Send + Sync + serde::Serialize + serde::Deserialize<'a> + 'static,
                $crate::adapters::Adapter<T>: $crate::adapters::TypeAdapter<$output>
            {
                (token.0, (std::boxed::Box::<$crate::adapters::Adapter<T>>::default(), std::any::TypeId::of::<T>()))
            }

            vec![
                $(
                    from($token),
                )*
            ]
            .into_iter()
            .collect()
        }
    };
}

/// Helper function to serialize an object
// pub fn serialize<V: Serialize + Any + Send + Sync, M, Output>(
//     key: &token::Key,
//     value: &dyn Any,
//     adapters: &HashMap<token::Key, Box<dyn TypeAdapter<Output> + Send + Sync>>,
// ) -> Result<Output, AdapterError> {
//     let adapter = adapters.get(key).ok_or(AdapterError::NoAdapter)?;
//     adapter.serialize(value)
// }

/// Helper function to deserialize data;
// pub fn deserialize<V: Serialize + Any + Send + Sync, M, Output>(
//     key: &token::Key,
//     value: &Output,
//     adapters: &HashMap<token::Key, Box<dyn TypeAdapter<Output> + Send + Sync>>,
// ) -> Result<Box<dyn Any + Send + Sync>, AdapterError> {
//     let adapter = adapters.get(key).ok_or(AdapterError::NoAdapter)?;
//     adapter.deserialize(value)
// }

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
