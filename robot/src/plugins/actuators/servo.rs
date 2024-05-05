use std::time::Duration;

use ahash::{HashMap, HashSet};
use bevy::prelude::*;
use common::{
    bundles::{PwmActuatorBundle, ServoBundle},
    components::{
        PwmChannel, PwmManualControl, PwmSignal, RobotId, ServoContribution, ServoDefinition,
        ServoMode, ServoTargets, Servos,
    },
    ecs_sync::{NetId, Replicate},
    events::{ResetServo, ResetServos},
};
use motor_math::motor_preformance::MotorData;

use crate::{
    config::{RobotConfig, Servo},
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
    let servos = &config.servo_config.servos;

    // TODO: Make this a bundle
    cmds.entity(robot.entity).insert((
        Servos {
            servos: servos.iter().map(|(name, _)| name.clone().into()).collect(),
        },
        ServoTargets::default(),
    ));

    for (
        name,
        Servo {
            pwm_channel,
            cameras,
        },
    ) in servos
    {
        cmds.spawn((
            ServoBundle {
                actuator: PwmActuatorBundle {
                    name: Name::new(name.clone()),
                    pwm_channel: PwmChannel(*pwm_channel),
                    pwm_signal: PwmSignal(Duration::from_micros(1500)),
                    robot: RobotId(robot.net_id),
                },
                servo: ServoDefinition {
                    cameras: cameras.iter().map(|it| it.clone().into()).collect(),
                },
                servo_mode: ServoMode::Velocity,
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
    servos: Query<(Entity, &Name, &ServoMode, &ServoDefinition, &RobotId)>,

    mut reset: EventReader<ResetServos>,
    mut reset_single: EventReader<ResetServo>,

    time: Res<Time<Real>>,
) {
    let Ok((robot, &net_id, last_positions)) = robot.get_single() else {
        return;
    };

    let mut all_inputs = HashMap::<_, f32>::default();

    for (&RobotId(robot_net_id), servo_contribution) in &servo_inputs {
        if robot_net_id != net_id {
            continue;
        }

        for (motor, input) in &servo_contribution.0 {
            *all_inputs.entry(motor.clone()).or_default() += *input;
        }
    }

    let servos_by_id = servos
        .iter()
        .map(|it| (it.1.as_str(), it))
        .collect::<HashMap<_, _>>();

    let mut full_reset = false;

    if !reset.is_empty() {
        full_reset = true;
        reset.clear();
    }

    let mut new_positions = last_positions.0.clone();

    for event in reset_single.read() {
        new_positions.insert(event.0.clone(), 0.0);
    }

    new_positions.extend(all_inputs.into_iter().flat_map(|(id, input)| {
        let (_, _, mode, _, _) = servos_by_id.get(&*id)?;

        match mode {
            ServoMode::Position => Some((id, input)),
            ServoMode::Velocity => {
                let last_position = if !full_reset {
                    last_positions.0.get(&id).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                Some((
                    id,
                    (last_position + input * time.delta_seconds()).clamp(-1.0, 1.0),
                ))
            }
        }
    }));

    for (id, position) in &new_positions {
        let Some((servo, ..)) = servos_by_id.get(&**id) else {
            continue;
        };

        let micros = 1500.0 + 400.0 * position.clamp(-1.0, 1.0);

        cmds.entity(*servo)
            .insert(PwmSignal(Duration::from_micros(micros as u64)));
    }

    cmds.entity(robot).insert(ServoTargets(new_positions));
}
