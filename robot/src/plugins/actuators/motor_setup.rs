use std::time::Duration;

use bevy::prelude::*;
use common::{
    bundles::{MotorBundle, PwmActuatorBundle, RobotActuatorBundle},
    components::{
        ActualForce, ActualMovement, Armed, CurrentDraw, MotorDefinition, Motors,
        MovementAxisMaximums, MovementCurrentCap, PwmChannel, PwmSignal, RobotId, TargetForce,
        TargetMovement,
    },
    ecs_sync::Replicate,
    types::units::Newtons,
};
use motor_math::{blue_rov::HeavyMotorId, solve::reverse, x3d::X3dMotorId};

use crate::{
    config::{MotorConfigDefinition, RobotConfig},
    plugins::core::robot::{LocalRobot, LocalRobotMarker},
};

use super::motor_math::MotorDataRes;

pub struct MotorSetupPlugin;

impl Plugin for MotorSetupPlugin {
    fn build(&self, app: &mut App) {
        // TODO(mid): Update motor config when motor definitions change
        app.add_systems(Startup, create_motors)
            .add_systems(Update, update_axis_maximums);
    }
}

fn create_motors(mut cmds: Commands, robot: Res<LocalRobot>, config: Res<RobotConfig>) {
    let (motors, motor_config) = config.motor_config.flatten();

    cmds.entity(robot.entity).insert(RobotActuatorBundle {
        movement_target: TargetMovement(Default::default()),
        movement_actual: ActualMovement(Default::default()),
        motor_config: Motors(motor_config),
        axis_maximums: MovementAxisMaximums(Default::default()),
        current_cap: MovementCurrentCap(config.motor_amperage_budget.into()),
        armed: Armed::Disarmed,
    });

    for (motor_id, motor, pwm_channel) in motors {
        let name = match config.motor_config {
            MotorConfigDefinition::X3d(_) => {
                format!(
                    "{:?} ({motor_id})",
                    X3dMotorId::try_from(motor_id).expect("Bad motor id for config")
                )
            }
            MotorConfigDefinition::BlueRov(_) => {
                format!(
                    "{:?} ({motor_id})",
                    HeavyMotorId::try_from(motor_id).expect("Bad motor id for config")
                )
            }
            MotorConfigDefinition::Custom(_) => format!("Motor {motor_id}"),
        };

        cmds.spawn((
            MotorBundle {
                actuator: PwmActuatorBundle {
                    name: Name::new(name),
                    pwm_channel: PwmChannel(pwm_channel),
                    pwm_signal: PwmSignal(Duration::from_micros(1500)),
                    robot: RobotId(robot.net_id),
                },
                motor: MotorDefinition(motor_id, motor),
                target_force: TargetForce(0.0f32.into()),
                actual_force: ActualForce(0.0f32.into()),
                current_draw: CurrentDraw(0.0f32.into()),
            },
            Replicate,
        ));
    }
}

fn update_axis_maximums(
    mut cmds: Commands,
    robot: Query<
        (Entity, &MovementCurrentCap, &Motors),
        (With<LocalRobotMarker>, Changed<MovementCurrentCap>),
    >,
    motor_data: Res<MotorDataRes>,
) {
    for (entity, current_cap, motor_config) in &robot {
        let motor_config = &motor_config.0;
        let motor_data = &motor_data.0;
        let current_cap = current_cap.0 .0;

        let maximums = reverse::axis_maximums(motor_config, motor_data, current_cap, 0.01)
            .into_iter()
            .map(|(key, value)| (key, Newtons(value)))
            .collect();

        info!("Updated motor axis maximums to {maximums:?} at {current_cap:.2}A");

        cmds.entity(entity).insert(MovementAxisMaximums(maximums));
    }
}
