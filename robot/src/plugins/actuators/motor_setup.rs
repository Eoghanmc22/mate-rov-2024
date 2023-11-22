use std::time::Duration;

use bevy::prelude::*;
use common::{
    bundles::{MotorBundle, PwmActuatorBundle, RobotActuatorBundle},
    components::{
        ActualForce, ActualMovement, Armed, CurrentDraw, MotorDefinition, Motors,
        MovementCurrentCap, PwmChannel, PwmSignal, RobotId, TargetForce, TargetMovement,
    },
    ecs_sync::NetworkId,
};

use crate::{config::RobotConfig, plugins::core::robot::LocalRobot};

pub struct MotorSetupPlugin;

impl Plugin for MotorSetupPlugin {
    fn build(&self, app: &mut App) {
        let robot: &LocalRobot = app.world.resource();
        let robot = robot.0;

        app.world.resource_scope(|world, config: Mut<RobotConfig>| {
            let motor_conf = config.motor_config.flatten();

            world.entity_mut(robot).insert(RobotActuatorBundle {
                movement_target: TargetMovement(Default::default()),
                movement_actual: ActualMovement(Default::default()),
                motor_config: Motors(motor_conf.1),
                current_cap: MovementCurrentCap(config.motor_amperage_budget.into()),
                armed: Armed::Disarmed,
            });

            for (motor_id, motor, pwm_channel) in motor_conf.0 {
                world.spawn(MotorBundle {
                    actuator: PwmActuatorBundle {
                        pwm_channel: PwmChannel(pwm_channel),
                        pwm_signal: PwmSignal(Duration::from_micros(1500)),
                        robot: RobotId(NetworkId::SINGLETON),
                    },
                    motor: MotorDefinition(motor_id, motor),
                    target_force: TargetForce(0.0f32.into()),
                    actual_force: ActualForce(0.0f32.into()),
                    current_draw: CurrentDraw(0.0f32.into()),
                });
            }
        });
    }
}
