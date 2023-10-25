use std::{
    any::{self, Any},
    borrow::Cow,
    fmt::{self, Debug},
    marker::PhantomData,
    sync::Arc,
};

/// Key that identifies a `Token`
pub type Key = Cow<'static, str>;

/// A handle to an untyped value
/// Allows us to reliably downcasting untyped values
pub struct Token<V: ?Sized, Meta>(pub Key, PhantomData<V>, PhantomData<Meta>);

impl<V, Meta> Token<V, Meta> {
    /// Creates a new [Token] with the key `key`
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into().into(), PhantomData, PhantomData)
    }

    /// Creates a new [Token] with the key `key`
    ///
    /// For use in const contexts
    pub const fn new_const(key: &'static str) -> Self {
        Self(Cow::Borrowed(key), PhantomData, PhantomData)
    }
}

impl<V: Any, Meta> Token<V, Meta> {
    /// Casts `value` to the type repersented by this token if possible
    pub fn downcast(&self, value: Box<dyn Any>) -> Result<V, Box<dyn Any>> {
        value.downcast().map(|it| *it)
    }

    /// Casts `value` to an [Arc] containing the type repersented by this token if possible
    pub fn downcast_arc(
        &self,
        value: Arc<dyn Any + Send + Sync>,
    ) -> Result<Arc<V>, Arc<dyn Any + Send + Sync>>
    where
        V: Send + Sync,
    {
        value.downcast()
    }
}

impl<V, M> Clone for Token<V, M> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData, PhantomData)
    }
}

impl<V, M> Debug for Token<V, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!(
            "Token<{}, {}>",
            any::type_name::<V>(),
            any::type_name::<M>()
        ))
        .field(&self.0)
        .finish()
    }
}

/// A trait allowing a token to be assoiciated with a type
///
/// Useful when a type only has one token assoiciated with it
pub trait Tokened {
    /// The [Token] assoiciated with this type
    const TOKEN: Token<Self, Self::TokenMeta>;
    /// That `TOKEN`'s token meta
    type TokenMeta;
}
