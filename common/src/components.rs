use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use bevy::{
    app::App,
    ecs::component::Component,
    reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize},
};
use glam::{Quat, Vec3A};
use motor_math::{solve::reverse::Axis, ErasedMotorId, Motor, MotorConfig, Movement};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::serde::ReflectSerdeAdapter,
    ecs_sync::{AppReplicateExt, NetId},
    types::{
        hw::{DepthFrame, InertialFrame, MagneticFrame, PwmChannelId},
        system::{ComponentTemperature, Cpu, Disk, Network, Process},
        units::{Amperes, Meters, Newtons, Volts},
    },
};

macro_rules! components {
    ($($name:ident),*) => {
        pub fn register_components(app: &mut App) {
            $(
                app.replicate::<$name>();
            )*
        }
    }
}

components! {
    Singleton,
    Robot,
    Surface,
    Orientation,
    Inertial,
    Magnetic,
    Depth,
    DepthTarget,
    OrientationTarget,
    Leak,
    RobotStatus,
    Armed,
    Camera,
    RobotId,
    Processes,
    LoadAverage,
    Networks,
    CpuTotal,
    Cores,
    Memory,
    Temperatures,
    Disks,
    Uptime,
    OperatingSystem,
    TargetForce,
    ActualForce,
    MotorDefinition,
    Motors,
    TargetMovement,
    ActualMovement,
    MeasuredVoltage,
    MovementContribution,
    MotorContribution,
    MovementAxisMaximums,
    MovementCurrentCap,
    CurrentDraw,
    PwmChannel,
    PwmSignal,
    PidConfig,
    PidResult
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Singleton;

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Robot;

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Surface;

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Orientation(pub Quat);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Inertial(pub InertialFrame);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Magnetic(pub MagneticFrame);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Depth(pub DepthFrame);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct DepthTarget(pub Meters);

/// Desired up vector
#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrientationTarget(pub Vec3A);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Leak(pub bool);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
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

#[derive(
    Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Eq, Default,
)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub enum Armed {
    Armed,
    #[default]
    Disarmed,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Eq)]
#[reflect(from_reflect = false)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Camera {
    // TODO(low): This bad
    #[reflect(ignore)]
    pub location: SocketAddr,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Eq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct RobotId(pub NetId);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Processes(pub Vec<Process>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Networks(pub Vec<Network>);

/// Total of each core
#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct CpuTotal(pub Cpu);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Cores(pub Vec<Cpu>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Memory {
    pub total_mem: u64,
    pub used_mem: u64,
    pub free_mem: u64,

    pub total_swap: u64,
    pub used_swap: u64,
    pub free_swap: u64,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Temperatures(pub Vec<ComponentTemperature>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Disks(pub Vec<Disk>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Uptime(pub Duration);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct OperatingSystem {
    pub name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub distro: Option<String>,
    pub host_name: Option<String>,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct TargetForce(pub Newtons);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActualForce(pub Newtons);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MotorDefinition(pub ErasedMotorId, pub Motor);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
#[reflect(from_reflect = false)]
pub struct Motors(
    // TODO(low): This bad
    #[reflect(ignore)] pub MotorConfig<ErasedMotorId>,
);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct TargetMovement(pub Movement);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActualMovement(pub Movement);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MeasuredVoltage(pub Volts);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct MovementContribution(pub Movement);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
#[reflect(from_reflect = false)]
pub struct MotorContribution(
    // TODO(low): This bad
    #[reflect(ignore)] pub BTreeMap<ErasedMotorId, Newtons>,
);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MovementAxisMaximums(
    // TODO(low): This bad
    #[reflect(ignore)] pub BTreeMap<Axis, Newtons>,
);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MovementCurrentCap(pub Amperes);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct CurrentDraw(pub Amperes);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct PwmChannel(pub PwmChannelId);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct PwmSignal(pub Duration);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct PidConfig {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,

    pub max_integral: f32,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(SerdeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct PidResult {
    pub p: f32,
    pub i: f32,
    pub d: f32,

    pub correction: f32,
}
