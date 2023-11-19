use bevy_ecs::bundle::Bundle;

use crate::{
    components::{
        ActualForce, ActualMovement, Armed, Camera, Cores, CpuTotal, CurrentDraw, Depth, Disks,
        Inertial, Leak, LoadAverage, Magnetic, MeasuredVoltage, Memory, MovementContribution,
        Networks, OperatingSystem, Orientation, Processes, RobotId, RobotMarker, RobotStatus,
        TargetForce, TargetMovement, Temperatures, Uptime,
    },
    ecs_sync::NetworkId,
};

#[derive(Default, Bundle, PartialEq)]
pub struct RobotBundle {
    pub core: RobotCoreBundle,
    pub sensors: RobotSensorBundle,
    pub system: RobotSystemBundle,
    pub actuators: RobotActuatorBundle,
    pub power: RobotPowerBundle,
}

#[derive(Default, Bundle, PartialEq)]
pub struct RobotCoreBundle {
    pub status: RobotStatus,
    pub net_id: NetworkId,

    pub marker: RobotMarker,
}

#[derive(Default, Bundle, PartialEq)]
pub struct RobotSensorBundle {
    pub orientation: Orientation,
    pub inertial: Inertial,
    pub mag: Magnetic,
    pub depth: Depth,
    pub leak: Leak,
}

#[derive(Default, Bundle, PartialEq)]
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

#[derive(Default, Bundle, PartialEq)]
pub struct RobotActuatorBundle {
    pub movement_target: TargetMovement,
    pub movement_actual: ActualMovement,
    pub armed: Armed,
}

#[derive(Default, Bundle, PartialEq)]
pub struct RobotPowerBundle {
    pub voltage: MeasuredVoltage,
    pub current_draw: CurrentDraw,
}

// TODO: Add transform?
#[derive(Bundle, PartialEq)]
pub struct CameraBundle {
    pub camera: Camera,

    pub robot: RobotId,
}

#[derive(Bundle, PartialEq)]
pub struct MotorBundle {
    pub target_force: TargetForce,
    pub actual_force: ActualForce,
    pub current_draw: CurrentDraw,

    pub robot: RobotId,
}

pub struct MovementContributionBundle {
    pub contribution: MovementContribution,

    pub robot: RobotId,
}
