use std::{net::SocketAddr, time::Duration};

use ahash::HashMap;
use bevy_ecs::component::Component;
use glam::{Quat, Vec3A};
use motor_math::{ErasedMotorId, Motor, MotorConfig, Movement};
use serde::{Deserialize, Serialize};

use crate::{
    adapters,
    ecs_sync::NetId,
    generate_adapters_components,
    token::{Token, Tokened},
    tokened,
    types::{
        hw::{DepthFrame, InertialFrame, MagneticFrame, PwmChannelId},
        system::{ComponentTemperature, Cpu, Disk, Network, Process},
        units::{Amperes, Meters, Newtons, Volts},
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
        Depth::TOKEN,
        DepthTarget::TOKEN,
        OrientationTarget::TOKEN,
        Leak::TOKEN,
        RobotStatus::TOKEN,
        Armed::TOKEN,
        Camera::TOKEN,
        RobotId::TOKEN,
        Processes::TOKEN,
        LoadAverage::TOKEN,
        Networks::TOKEN,
        CpuTotal::TOKEN,
        Cores::TOKEN,
        Memory::TOKEN,
        Temperatures::TOKEN,
        Disks::TOKEN,
        Uptime::TOKEN,
        OperatingSystem::TOKEN,
        TargetForce::TOKEN,
        ActualForce::TOKEN,
        MotorDefinition::TOKEN,
        Motors::TOKEN,
        TargetMovement::TOKEN,
        ActualMovement::TOKEN,
        MeasuredVoltage::TOKEN,
        ActuatorContributionMarker::TOKEN,
        MovementContribution::TOKEN,
        MotorContribution::TOKEN,
        MovementCurrentCap::TOKEN,
        CurrentDraw::TOKEN,
        PwmChannel::TOKEN,
        PwmSignal::TOKEN,
        PidConfig::TOKEN,
        PidResult::TOKEN
    }
}
tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot")]
    pub struct RobotMarker(pub String);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
    #[token("robot.orientation")]
    pub struct Orientation(pub Quat);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    #[token("robot.sensors.inertial")]
    pub struct Inertial(pub InertialFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    #[token("robot.sensors.magnetic")]
    pub struct Magnetic(pub MagneticFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    #[token("robot.sensors.depth")]
    pub struct Depth(pub DepthFrame);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    #[token("robot.sensors.depth.hold")]
    pub struct DepthTarget(pub Meters);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
    #[token("robot.sensors.orientation.hold")]
    // Desired up vector
    pub struct OrientationTarget(pub Vec3A);
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
    pub struct RobotId(pub NetId);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.system.processes")]
    pub struct Processes(pub Vec<Process>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.system.uptime")]
    pub struct Uptime(pub Duration);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.motor.goal")]
    pub struct TargetForce(pub Newtons);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.motor.real")]
    pub struct ActualForce(pub Newtons);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.motor")]
    pub struct MotorDefinition(pub ErasedMotorId, pub Motor);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.motors")]
    pub struct Motors(pub MotorConfig<ErasedMotorId>);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.movement.goal")]
    pub struct TargetMovement(pub Movement);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.movement.real")]
    pub struct ActualMovement(pub Movement);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.voltage")]
    pub struct MeasuredVoltage(pub Volts);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.movement.marker")]
    pub struct ActuatorContributionMarker(pub String);
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
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.movement.cap")]
    pub struct MovementCurrentCap(pub Amperes);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.current")]
    pub struct CurrentDraw(pub Amperes);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.pwm.id")]
    pub struct PwmChannel(pub PwmChannelId);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.pwm")]
    pub struct PwmSignal(pub Duration);
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    #[token("robot.pid.config")]
    pub struct PidConfig {
        pub kp: f32,
        pub ki: f32,
        pub kd: f32,

        pub max_integral: f32,
    }
}

tokened! {
    #[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[token("robot.pid.result")]
    pub struct PidResult {
        pub p: f32,
        pub i: f32,
        pub d: f32,

        pub correction: f32,
    }
}
