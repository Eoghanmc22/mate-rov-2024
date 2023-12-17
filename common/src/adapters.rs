//! Infrastructure to serialize and recover data

use std::{any::Any, marker::PhantomData, sync::Arc};

use anyhow::Context;
use bevy_ecs::ptr::{OwningPtr, Ptr};
use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::instrument;

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
pub type BackingType = Arc<Vec<u8>>;
impl<T> TypeAdapter<BackingType> for Adapter<T>
where
    for<'a> T: Serialize + Deserialize<'a> + Any + Send + Sync,
{
    #[instrument(level = "trace", skip_all)]
    unsafe fn serialize(&self, ptr: Ptr<'_>) -> Result<BackingType, AdapterError> {
        let val = unsafe { ptr.deref::<T>() };
        options()
            .serialize(val)
            .context("Bincode error")
            .map(Into::into)
            .map_err(AdapterError::SerializationError)
    }

    #[instrument(level = "trace", skip_all)]
    fn deserialize(
        &self,
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

/// The serializeation settings used
fn options() -> impl Options {
    DefaultOptions::new()
}

/// Macros to generate adapter lookup tables
#[macro_export]
macro_rules! generate_adapters_components {
    (name = $name:ident, output = $output:ty, tokens = { $($token:expr),* }) => {
        pub fn $name(world: &mut bevy_ecs::world::World) -> ahash::HashMap<$crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, bevy_ecs::component::ComponentDescriptor, fn(&mut bevy_ecs::world::EntityWorldMut))>
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

            let vec = vec![
                $(
                    from($token),
                )*
            ];
            let len = vec.len();

            let map: ahash::HashMap<_, _> = vec.into_iter().collect();

            assert_eq!(len, map.len());

            map
        }
    };
}
#[macro_export]
macro_rules! tokened {
    ($(#[derive $traits:tt])? #[token($token:literal)] $vis:vis $ident:ident $name:ident $trailing1:tt $($trailing2:tt)?) => {
        $(#[derive $traits])?
        $vis $ident $name $trailing1 $($trailing2)?

        impl Tokened for $name {
            const TOKEN: Token<Self, Self::TokenMeta> = Token::new_const($token);

            type TokenMeta = ();
        }
    }
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
