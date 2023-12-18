use std::{net::SocketAddr, time::Duration};

use ahash::HashMap;
use bevy::{
    ecs::component::Component,
    reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize},
};
use glam::{Quat, Vec3A};
use motor_math::{ErasedMotorId, Motor, MotorConfig, Movement};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::ReflectTypeAdapter,
    ecs_sync::NetId,
    // generate_adapters_components,
    types::{
        hw::{DepthFrame, InertialFrame, MagneticFrame, PwmChannelId},
        system::{ComponentTemperature, Cpu, Disk, Network, Process},
        units::{Amperes, Meters, Newtons, Volts},
    },
};

// generate_adapters_components! {
//     name = adapters_components,
//     output = adapters::BackingType,
//     tokens = {
//         RobotMarker,
//         Orientation,
//         Inertial,
//         Magnetic,
//         Depth,
//         DepthTarget,
//         OrientationTarget,
//         Leak,
//         RobotStatus,
//         Armed,
//         Camera,
//         RobotId,
//         Processes,
//         LoadAverage,
//         Networks,
//         CpuTotal,
//         Cores,
//         Memory,
//         Temperatures,
//         Disks,
//         Uptime,
//         OperatingSystem,
//         TargetForce,
//         ActualForce,
//         MotorDefinition,
//         Motors,
//         TargetMovement,
//         ActualMovement,
//         MeasuredVoltage,
//         ActuatorContributionMarker,
//         MovementContribution,
//         MotorContribution,
//         MovementCurrentCap,
//         CurrentDraw,
//         PwmChannel,
//         PwmSignal,
//         PidConfig,
//         PidResult
//     }
// }

// /// Macros to generate adapter lookup tables
// #[macro_export]
// macro_rules! generate_adapters_components {
//     (name = $name:ident, output = $output:ty, tokens = { $($token:expr),* }) => {
//         pub fn $name(world: &mut bevy_ecs::world::World) -> ahash::HashMap<$crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, bevy_ecs::component::ComponentDescriptor, fn(&mut bevy_ecs::world::EntityWorldMut))>
//         {
//             fn from<T: bevy_ecs::component::Component, O>(token: $crate::token::Token<T, O>) -> ($crate::token::Key, (std::boxed::Box<dyn $crate::adapters::TypeAdapter<$output> + Send + Sync>, bevy_ecs::component::ComponentDescriptor, fn(&mut bevy_ecs::world::EntityWorldMut)))
//             where
//                 for<'a> T: Send + Sync + serde::Serialize + serde::Deserialize<'a> + 'static,
//                 $crate::adapters::Adapter<T>: $crate::adapters::TypeAdapter<$output>
//             {
//                 (token.0, (std::boxed::Box::<$crate::adapters::Adapter<T>>::default(), bevy_ecs::component::ComponentDescriptor::new::<T>(), |entity| {
//                     entity.remove::<T>();
//                 }))
//             }
//
//             let vec = vec![
//                 $(
//                     from($token),
//                 )*
//             ];
//             let len = vec.len();
//
//             let map: ahash::HashMap<_, _> = vec.into_iter().collect();
//
//             assert_eq!(len, map.len());
//
//             map
//         }
//     };
// }
// #[macro_export]
// macro_rules! tokened {
//     ($(#[derive $traits:tt])? #[token($token:literal)] $vis:vis $ident:ident $name:ident $trailing1:tt $($trailing2:tt)?) => {
//         $(#[derive $traits])?
//         $vis $ident $name $trailing1 $($trailing2)?
//
//         impl Tokened for $name {
//             const TOKEN: Token<Self, Self::TokenMeta> = Token::new_const($token);
//
//             type TokenMeta = ();
//         }
//     }
// }

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct RobotMarker(pub String);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Orientation(pub Quat);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Inertial(pub InertialFrame);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Magnetic(pub MagneticFrame);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Depth(pub DepthFrame);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct DepthTarget(pub Meters);

/// Desired up vector
#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrientationTarget(pub Vec3A);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Leak(pub bool);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
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
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub enum Armed {
    Armed,
    #[default]
    Disarmed,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Eq)]
#[reflect(from_reflect = false)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Camera {
    pub name: String,
    // TODO: This bad
    #[reflect(ignore)]
    pub location: SocketAddr,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Copy, Clone, PartialEq, Eq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct RobotId(pub NetId);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Processes(pub Vec<Process>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Networks(pub Vec<Network>);

/// Total of each core
#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct CpuTotal(pub Cpu);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Cores(pub Vec<Cpu>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Memory {
    pub total_mem: u64,
    pub used_mem: u64,
    pub free_mem: u64,

    pub total_swap: u64,
    pub used_swap: u64,
    pub free_swap: u64,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Temperatures(pub Vec<ComponentTemperature>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Disks(pub Vec<Disk>);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct Uptime(pub Duration);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct OperatingSystem {
    pub name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub distro: Option<String>,
    pub host_name: Option<String>,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct TargetForce(pub Newtons);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActualForce(pub Newtons);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MotorDefinition(pub ErasedMotorId, pub Motor);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
#[reflect(from_reflect = false)]
pub struct Motors(
    // TODO: This bad
    #[reflect(ignore)] pub MotorConfig<ErasedMotorId>,
);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct TargetMovement(pub Movement);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActualMovement(pub Movement);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MeasuredVoltage(pub Volts);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActuatorContributionMarker(pub String);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct MovementContribution(pub Movement);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
#[reflect(from_reflect = false)]
pub struct MotorContribution(
    // TODO: This bad
    #[reflect(ignore)] pub HashMap<ErasedMotorId, Newtons>,
);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct MovementCurrentCap(pub Amperes);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct CurrentDraw(pub Amperes);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct PwmChannel(pub PwmChannelId);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct PwmSignal(pub Duration);

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct PidConfig {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,

    pub max_integral: f32,
}

#[derive(Component, Serialize, Deserialize, Reflect, Debug, Clone, PartialEq)]
#[reflect(TypeAdapter, Serialize, Deserialize, Debug, PartialEq)]
pub struct PidResult {
    pub p: f32,
    pub i: f32,
    pub d: f32,

    pub correction: f32,
}
