use bevy::{core::Name, ecs::bundle::Bundle, transform::components::Transform};

use crate::components::{
    ActualForce, ActualMovement, Armed, Camera, Cores, CpuTotal, CurrentDraw, Depth, Disks,
    Inertial, Leak, LoadAverage, Magnetic, MeasuredVoltage, Memory, MotorDefinition, Motors,
    MovementAxisMaximums, MovementContribution, MovementCurrentCap, Networks, OperatingSystem,
    Orientation, Processes, PwmChannel, PwmSignal, Robot, RobotId, RobotStatus, ServoDefinition,
    ServoMode, ServoTargets, TargetForce, TargetMovement, Temperatures, Uptime,
};

#[derive(Bundle, PartialEq)]
pub struct RobotBundle {
    pub core: RobotCoreBundle,
    pub sensors: RobotSensorBundle,
    pub system: RobotSystemBundle,
    pub actuators: RobotActuatorBundle,
    pub power: RobotPowerBundle,
    // pub manual: Option<PwmManualControl>,
}

#[derive(Bundle, PartialEq)]
pub struct RobotCoreBundle {
    pub marker: Robot,
    pub status: RobotStatus,
    pub name: Name,

    pub robot_id: RobotId,
}

#[derive(Bundle, PartialEq)]
pub struct RobotSensorBundle {
    pub orientation: Orientation,
    pub inertial: Inertial,
    pub mag: Magnetic,
    pub depth: Depth,
    pub leak: Leak,
}

#[derive(Bundle, PartialEq)]
pub struct RobotSystemBundle {
    pub processes: Processes,
    pub load_average: LoadAverage,
    pub networks: Networks,
    pub cpu: CpuTotal,
    pub cores: Cores,
    pub memory: Memory,
    pub temps: Temperatures,
    pub disks: Disks,
    pub uptime: Uptime,
    pub os: OperatingSystem,
}

#[derive(Bundle, PartialEq)]
pub struct RobotActuatorBundle {
    pub movement_target: TargetMovement,
    pub movement_actual: ActualMovement,

    pub motor_config: Motors,
    pub axis_maximums: MovementAxisMaximums,
    pub current_cap: MovementCurrentCap,

    pub armed: Armed,
}

// TODO(mid): Sensor not implemented
#[derive(Bundle, PartialEq)]
pub struct RobotPowerBundle {
    pub voltage: MeasuredVoltage,
    pub current_draw: CurrentDraw,
}

#[derive(Bundle, PartialEq)]
pub struct CameraBundle {
    pub name: Name,
    pub camera: Camera,
    pub transform: Transform,

    pub robot: RobotId,
}

#[derive(Bundle, PartialEq)]
pub struct MotorBundle {
    pub actuator: PwmActuatorBundle,

    pub motor: MotorDefinition,

    pub target_force: TargetForce,
    pub actual_force: ActualForce,
    pub current_draw: CurrentDraw,
}

#[derive(Bundle, PartialEq)]
pub struct ServoBundle {
    pub actuator: PwmActuatorBundle,

    pub servo: ServoDefinition,
    pub servo_mode: ServoMode,
}

#[derive(Bundle, PartialEq)]
pub struct PwmActuatorBundle {
    pub name: Name,
    pub pwm_channel: PwmChannel,
    pub pwm_signal: PwmSignal,

    pub robot: RobotId,
}

#[derive(Bundle, PartialEq)]
pub struct MovementContributionBundle {
    pub name: Name,

    pub contribution: MovementContribution,

    pub robot: RobotId,
}
