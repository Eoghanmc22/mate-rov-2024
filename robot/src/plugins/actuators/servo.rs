use std::{collections::BTreeMap, time::Duration};

use ahash::HashMap;
use bevy::prelude::*;
use common::{
    bundles::{MotorBundle, PwmActuatorBundle, RobotActuatorBundle},
    components::{
        ActualForce, ActualMovement, Armed, CurrentDraw, MotorDefinition, Motors,
        MovementAxisMaximums, MovementCurrentCap, PwmChannel, PwmManualControl, PwmSignal, RobotId,
        ServoContribution, ServoDefinition, ServoMode, ServoTargets, TargetForce, TargetMovement,
    },
    ecs_sync::{NetId, Replicate},
};
use motor_math::{blue_rov::HeavyMotorId, motor_preformance::MotorData, x3d::X3dMotorId};

use crate::{
    config::{MotorConfigDefinition, RobotConfig},
    plugins::core::robot::{LocalRobot, LocalRobotMarker},
};

pub struct ServoPlugin;

impl Plugin for ServoPlugin {
    fn build(&self, app: &mut App) {
        // TODO(mid): Update motor config when motor definitions change
        app.add_systems(Startup, create_servos)
            .add_systems(Update, handle_servo_input);
    }
}

#[derive(Resource)]
pub struct MotorDataRes(pub MotorData);

fn create_servos(mut cmds: Commands, robot: Res<LocalRobot>, config: Res<RobotConfig>) {
    let (motors, motor_config) = config.motor_config.flatten(config.center_of_mass);

    info!("Generating motor config");

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

fn handle_servo_input(
    mut cmds: Commands,

    robot: Query<
        (Entity, &NetId, &ServoTargets),
        (With<LocalRobotMarker>, Without<PwmManualControl>),
    >,
    servo_inputs: Query<(&RobotId, &ServoContribution)>,
    // TODO
    servos: Query<(Entity, &ServoMode, &ServoDefinition, &RobotId)>,

    time: Res<Time<Real>>,
) {
    let Ok((robot, &net_id, last_positions)) = robot.get_single() else {
        return;
    };

    let mut all_inputs = HashMap::<_, f32>::default();

    for (&RobotId(robot_net_id), servo_contribution) in &servo_inputs {
        if robot_net_id == net_id {
            continue;
        }

        for (motor, input) in &servo_contribution.0 {
            *all_inputs.entry(*motor).or_default() += *input;
        }
    }

    let servos_by_id = servos
        .iter()
        .map(|it| (it.2.id, it))
        .collect::<HashMap<_, _>>();

    let new_positions = all_inputs
        .into_iter()
        .flat_map(|(id, input)| {
            let (_, mode, _, _) = servos_by_id.get(&id)?;

            match mode {
                ServoMode::Position => Some((id, input)),
                ServoMode::Velocity => {
                    let last_position = last_positions.0.get(&id)?;
                    Some((id, last_position + input * time.delta_seconds()))
                }
            }
        })
        .collect::<BTreeMap<_, _>>();

    for (id, position) in &new_positions {
        let Some((servo, ..)) = servos_by_id.get(id) else {
            continue;
        };

        let micros = 1500.0 + 400.0 * position;

        cmds.entity(*servo)
            .insert(PwmSignal(Duration::from_micros(micros as u64)));
    }

    cmds.entity(robot).insert(ServoTargets(new_positions));
}
