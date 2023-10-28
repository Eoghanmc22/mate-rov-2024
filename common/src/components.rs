use bevy_ecs::component::Component;
use glam::Quat;
use serde::{Deserialize, Serialize};

use crate::{
    adapters, generate_adapters_components, generate_adapters_resources,
    token::{Token, Tokened},
    tokened,
    types::sensors::{DepthFrame, InertialFrame, MagneticFrame},
};

generate_adapters_components! {
    name = adapters_components,
    output = adapters::BackingType,
    tokens = {
        RobotMarker::TOKEN,
        Orientation::TOKEN,
        Inertial::TOKEN,
        Magnetic::TOKEN
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
    pub struct Inertial(pub InertialFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot.sensors.magnetic")]
    pub struct Magnetic(pub MagneticFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot.sensors.depth")]
    pub struct Depth(pub DepthFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug)]
    #[token("robot.sensors.leak")]
    pub struct Leak(pub bool);
}
