use bevy_ecs::{component::Component, system::Resource};
use serde::{Deserialize, Serialize};

use crate::{
    adapters, generate_adapters_components, generate_adapters_resources,
    token::{Token, Tokened},
};

generate_adapters_components! {
    name = adapters_components,
    output = adapters::BackingType,
    tokens = {
        Test::TOKEN
    }
}
generate_adapters_resources! {
    name = adapters_resources,
    output = adapters::BackingType,
    tokens = {
        Test::TOKEN
    }
}

#[derive(Resource, Component, Serialize, Deserialize)]
pub struct Test(pub u8);

impl Tokened for Test {
    const TOKEN: Token<Self, Self::TokenMeta> = Token::new_const("test");

    type TokenMeta = ();
}
