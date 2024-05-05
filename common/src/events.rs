use std::borrow::Cow;

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
    ResyncCameras,
    CalibrateSeaLevel,
    ResetYaw,
    ResetServos,
    ResetServo
}

#[derive(Event, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResyncCameras;

#[derive(Event, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct CalibrateSeaLevel;

#[derive(Event, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResetYaw;

#[derive(Event, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResetServos;

#[derive(Event, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResetServo(pub Cow<'static, str>);
