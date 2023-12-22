use std::time::Duration;

use bevy::prelude::*;
use common::{
    bundles::{MotorBundle, PwmActuatorBundle, RobotActuatorBundle},
    components::{
        ActualForce, ActualMovement, Armed, CurrentDraw, MotorDefinition, Motors,
        MovementCurrentCap, PwmChannel, PwmSignal, RobotId, TargetForce, TargetMovement,
    },
    ecs_sync::Replicate,
};
use motor_math::{blue_rov::HeavyMotorId, x3d::X3dMotorId};

use crate::{
    config::{MotorConfigDefinition, RobotConfig},
    plugins::core::robot::LocalRobot,
};

pub struct MotorSetupPlugin;

impl Plugin for MotorSetupPlugin {
    fn build(&self, app: &mut App) {
        // TODO(mid): Update motor config when motor definitions change
        app.add_systems(Startup, create_motors);
    }
}

fn create_motors(mut cmds: Commands, robot: Res<LocalRobot>, config: Res<RobotConfig>) {
    let motor_conf = config.motor_config.flatten();

    cmds.entity(robot.entity).insert(RobotActuatorBundle {
        movement_target: TargetMovement(Default::default()),
        movement_actual: ActualMovement(Default::default()),
        motor_config: Motors(motor_conf.1),
        current_cap: MovementCurrentCap(config.motor_amperage_budget.into()),
        armed: Armed::Disarmed,
    });

    for (motor_id, motor, pwm_channel) in motor_conf.0 {
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
