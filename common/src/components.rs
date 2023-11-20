use std::{net::SocketAddr, time::Duration};

use ahash::HashMap;
use bevy_ecs::component::Component;
use glam::Quat;
use motor_math::{ErasedMotorId, Motor, Movement};
use serde::{Deserialize, Serialize};

use crate::{
    adapters,
    ecs_sync::NetworkId,
    generate_adapters_components, generate_adapters_resources,
    token::{Token, Tokened},
    tokened,
    types::{
        sensors::{DepthFrame, InertialFrame, MagneticFrame},
        system::{ComponentTemperature, Cpu, Disk, Network, Process},
        units::{Amperes, Newtons, Volts},
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
        RobotId::TOKEN,
        Processes::TOKEN,
        Cores::TOKEN,
        Networks::TOKEN,
        LoadAverage::TOKEN,
        Cores::TOKEN,
        Memory::TOKEN,
        Temperatures::TOKEN,
        Disks::TOKEN,
        Uptime::TOKEN,
        OperatingSystem::TOKEN
    }
}
generate_adapters_resources! {
    name = adapters_resources,
    output = adapters::BackingType,
    tokens = {
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot")]
    pub struct RobotMarker;
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.orientation")]
    pub struct Orientation(pub Quat);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.sensors.inertial")]
    pub struct Inertial(pub InertialFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.sensors.magnetic")]
    pub struct Magnetic(pub MagneticFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.sensors.depth")]
    pub struct Depth(pub DepthFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.sensors.leak")]
    pub struct Leak(pub bool);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.status")]
    pub enum RobotStatus {
        /// No peer is connected
        #[default]
        NoPeer,
        /// Peer is connected and robot is disarmed
        Disarmed,
        /// Peer is connected and robot is armed
        Ready,
        /// The robot is moving, includes speed
        Moving(Newtons),
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
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
    #[token("robot.id")]
    pub struct RobotId(pub NetworkId);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.processes")]
    pub struct Processes(pub Vec<Process>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.load_average")]
    pub struct LoadAverage {
        pub one_min: f64,
        pub five_min: f64,
        pub fifteen_min: f64
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.networks")]
    pub struct Networks(pub Vec<Network>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.cpu")]
    // Total of each core
    pub struct CpuTotal(pub Cpu);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.cores")]
    pub struct Cores(pub Vec<Cpu>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.mem")]
    pub struct Memory {
        pub total_mem: u64,
        pub used_mem: u64,
        pub free_mem: u64,

        pub total_swap: u64,
        pub used_swap: u64,
        pub free_swap: u64,
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.temps")]
    pub struct Temperatures(pub Vec<ComponentTemperature>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.disks")]
    pub struct Disks(pub Vec<Disk>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.uptime")]
    pub struct Uptime(pub Duration);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.os")]
    pub struct OperatingSystem{
        pub name: Option<String>,
        pub kernel_version: Option<String>,
        pub os_version: Option<String>,
        pub distro: Option<String>,
        pub host_name: Option<String>,
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.motor.goal")]
    pub struct TargetForce(pub Newtons);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.motor.real")]
    pub struct ActualForce(pub Newtons);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.motor")]
    pub struct MotorDefinition(pub Motor);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.movement.goal")]
    pub struct TargetMovement(pub Movement);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.movement.real")]
    pub struct ActualMovement(pub Movement);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.voltage")]
    pub struct MeasuredVoltage(pub Volts);
}
tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.movement")]
    pub struct MovementContribution(pub Movement);
}
tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.movement.raw")]
    pub struct MotorContribution(pub HashMap<ErasedMotorId, Newtons>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.current")]
    pub struct CurrentDraw(pub Amperes);
}
