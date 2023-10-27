use bevy_ecs::{component::Component, system::Resource};
use glam::Quat;
use serde::{Deserialize, Serialize};

use crate::{
    adapters, generate_adapters_components, generate_adapters_resources,
    token::{Token, Tokened},
    tokened,
    types::sensors::{InertialFrame, MagFrame},
};

generate_adapters_components! {
    name = adapters_components,
    output = adapters::BackingType,
    tokens = {
        RobotMarker::TOKEN,
        Orientation::TOKEN,
        RawInertial::TOKEN,
        RawMagnetic::TOKEN
    }
}
generate_adapters_resources! {
    name = adapters_resources,
    output = adapters::BackingType,
    tokens = {
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot")]
    pub struct RobotMarker;
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot.orientation")]
    pub struct Orientation(pub Quat);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot.sensors.inertial")]
    pub struct RawInertial(pub InertialFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot.sensors.magnetic")]
    pub struct RawMagnetic(pub MagFrame);
}
