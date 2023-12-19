use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use common::{
    bundles::MovementContributionBundle,
    components::{
        ActuatorContributionMarker, MovementContribution, Orientation, OrientationTarget,
        PidConfig, PidResult, RobotId,
    },
    ecs_sync::Replicate,
    types::utils::PidController,
};
use glam::{vec3a, Vec3A};
use motor_math::Movement;

use crate::plugins::core::robot::LocalRobot;

pub struct StabilizePlugin;

impl Plugin for StabilizePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, stabalize_system);

        let robot_net_id = app.world.resource::<LocalRobot>().net_id;

        let pitch = app
            .world
            .spawn((
                Name::new("Stabalize Pitch"),
                MovementContributionBundle {
                    marker: ActuatorContributionMarker("Stabalize Pitch".to_owned()),
                    contribution: MovementContribution(Movement::default()),
                    robot: RobotId(robot_net_id),
                },
                // TODO: Tune
                // TODO: Load from disk?
                PidConfig {
                    kp: 1.0,
                    ki: 0.0,
                    kd: 0.0,
                    max_integral: 0.0,
                },
                Replicate,
            ))
            .id();

        let roll = app
            .world
            .spawn((
                Name::new("Stabalize Roll"),
                MovementContributionBundle {
                    marker: ActuatorContributionMarker("Stabalize Roll".to_owned()),
                    contribution: MovementContribution(Movement::default()),
                    robot: RobotId(robot_net_id),
                },
                // TODO: Tune
                // TODO: Load from disk?
                PidConfig {
                    kp: 1.0,
                    ki: 0.0,
                    kd: 0.0,
                    max_integral: 0.0,
                },
                Replicate,
            ))
            .id();

        app.insert_resource(StabilizeState {
            pitch,
            pitch_controller: PidController::default(),
            roll,
            roll_controller: PidController::default(),
        });
    }
}

#[derive(Resource)]
struct StabilizeState {
    pitch: Entity,
    pitch_controller: PidController,

    roll: Entity,
    roll_controller: PidController,
}

pub fn stabalize_system(
    mut cmds: Commands,
    robot: Res<LocalRobot>,
    mut state: ResMut<StabilizeState>,
    robot_query: Query<(&Orientation, &OrientationTarget)>,
    entity_query: Query<&PidConfig>,
    time: Res<Time<Real>>,
) {
    let robot = robot_query.get(robot.entity);
    let pitch_pid_config = entity_query.get(state.pitch).unwrap();
    let roll_pid_config = entity_query.get(state.roll).unwrap();

    if let Ok((orientation, orientation_target)) = robot {
        let observed_up = orientation.0 * Vec3A::Z;
        let target_up = orientation_target.0;

        // TODO: Is this any good?
        let error = Quat::from_rotation_arc(observed_up.into(), target_up.into());
        let pitch_error = instant_twist(error, orientation.0 * Vec3A::X).to_degrees();
        let roll_error = instant_twist(error, orientation.0 * Vec3A::Y).to_degrees();

        let res_pitch = state
            .pitch_controller
            .update(pitch_error, pitch_pid_config, time.delta());
        let res_roll = state
            .roll_controller
            .update(roll_error, roll_pid_config, time.delta());

        let pitch_movement = Movement {
            force: Vec3A::ZERO,
            torque: Vec3A::X * pitch_error,
        };

        let roll_movement = Movement {
            force: Vec3A::ZERO,
            torque: Vec3A::Y * roll_error,
        };

        cmds.entity(state.pitch)
            .insert((MovementContribution(pitch_movement), res_pitch));
        cmds.entity(state.roll)
            .insert((MovementContribution(roll_movement), res_roll));
    } else {
        cmds.entity(state.pitch)
            .remove::<(MovementContribution, PidResult)>();
        cmds.entity(state.roll)
            .remove::<(MovementContribution, PidResult)>();
    }
}

fn instant_twist(q: Quat, twist_axis: Vec3A) -> f32 {
    let rotation_axis = vec3a(q.x, q.y, q.z);

    let sign = rotation_axis.dot(twist_axis).signum();
    let projected = rotation_axis.project_onto(twist_axis);
    let twist = Quat::from_xyzw(projected.x, projected.y, projected.z, q.w).normalize() * sign;

    let angle = twist.w.acos() * 2.0;
    normalize_angle(angle)
}

fn normalize_angle(angle: f32) -> f32 {
    let wrapped_angle = modf(angle, TAU);
    if wrapped_angle > PI {
        wrapped_angle - TAU
    } else {
        wrapped_angle
    }
}

fn modf(a: f32, b: f32) -> f32 {
    (a % b + b) % b
}
