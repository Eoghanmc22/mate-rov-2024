use bevy_ecs::component::Component;

#[derive(Component, Hash, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId(pub(crate) u128);

impl NetworkId {
    pub fn random() -> Self {
        Self(rand::random())
    }
}
