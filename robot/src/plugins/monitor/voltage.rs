use bevy::prelude::*;
use common::{
    bundles::RobotCoreBundle,
    components::{CurrentDraw, MeasuredVoltage, Robot, RobotId, RobotStatus, Singleton},
    ecs_sync::{NetId, Replicate},
    InstanceName,
};

use crate::plugins::core::robot::LocalRobotMarker;

pub struct VoltagePlugin;

impl Plugin for VoltagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, check_voltage);
    }
}

fn check_voltage(robot: Query<(&MeasuredVoltage, &CurrentDraw), With<LocalRobotMarker>>) {
    for (voltage, current) in &robot {
        if voltage.0 .0 < 10.0 {
            warn!("Low Voltage: {}, {}", voltage.0, current.0);
        }
    }
}
