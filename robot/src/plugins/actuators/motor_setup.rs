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

use crate::{config::RobotConfig, plugins::core::robot::LocalRobot};

pub struct MotorSetupPlugin;

impl Plugin for MotorSetupPlugin {
    fn build(&self, app: &mut App) {
        app.world.resource_scope(|world, robot: Mut<LocalRobot>| {
            world.resource_scope(|world, config: Mut<RobotConfig>| {
                let motor_conf = config.motor_config.flatten();

                world.entity_mut(robot.entity).insert(RobotActuatorBundle {
                    movement_target: TargetMovement(Default::default()),
                    movement_actual: ActualMovement(Default::default()),
                    motor_config: Motors(motor_conf.1),
                    current_cap: MovementCurrentCap(config.motor_amperage_budget.into()),
                    armed: Armed::Disarmed,
                });

                for (motor_id, motor, pwm_channel) in motor_conf.0 {
                    world.spawn((
                        MotorBundle {
                            actuator: PwmActuatorBundle {
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
            });
        });
    }
}
