use std::net::SocketAddr;

use bevy_ecs::component::Component;
use glam::Quat;
use serde::{Deserialize, Serialize};

use crate::{
    adapters,
    ecs_sync::NetworkId,
    generate_adapters_components, generate_adapters_resources,
    token::{Token, Tokened},
    tokened,
    types::{
        sensors::{DepthFrame, InertialFrame, MagneticFrame},
        units::Percent,
    },
};

generate_adapters_components! {
    name = adapters_components,
    output = adapters::BackingType,
    tokens = {
        RobotMarker::TOKEN,
        Orientation::TOKEN,
        Inertial::TOKEN,
        Magnetic::TOKEN,
        RobotStatus::TOKEN,
        Armed::TOKEN,
        Camera::TOKEN,
        RobotId::TOKEN
    }
}
generate_adapters_resources! {
    name = adapters_resources,
    output = adapters::BackingType,
    tokens = {
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot")]
    pub struct RobotMarker;
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot.orientation")]
    pub struct Orientation(pub Quat);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot.sensors.inertial")]
    pub struct Inertial(pub InertialFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot.sensors.magnetic")]
    pub struct Magnetic(pub MagneticFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot.sensors.depth")]
    pub struct Depth(pub DepthFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot.sensors.leak")]
    pub struct Leak(pub bool);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    #[token("robot.status")]
    pub enum RobotStatus {
        /// No peer is connected
        NoPeer,
        /// Peer is connected and robot is disarmed
        Disarmed,
        /// Peer is connected and robot is armed
        Ready,
        /// The robot is moving, includes speed
        Moving(Percent),
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Default)]
    #[token("robot.armed")]
    pub enum Armed {
        Armed,
        #[default]
        Disarmed,
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    #[token("robot.camera")]
    pub struct Camera {
        pub name: String,
        pub location: SocketAddr,
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone)]
    #[token("robot.id")]
    pub struct RobotId(pub NetworkId);
}
