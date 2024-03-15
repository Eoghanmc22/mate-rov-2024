use bevy::{
    app::App,
    ecs::event::Event,
    reflect::{Reflect, ReflectDeserialize, ReflectSerialize},
};
use serde::{Deserialize, Serialize};

use crate::{adapters::serde::ReflectSerdeAdapter, ecs_sync::AppReplicateExt};

macro_rules! events {
    ($($name:ident),*) => {
        pub fn register_events(app: &mut App) {
            $(
                app.replicate_event::<$name>();
            )*
        }
    }
}

events! {
    ResyncCameras
}

#[derive(Event, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResyncCameras;
