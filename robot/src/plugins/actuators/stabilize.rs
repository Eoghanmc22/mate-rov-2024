use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use common::{
    bundles::MovementContributionBundle,
    components::{
        Armed, MovementContribution, Orientation, OrientationTarget, PidConfig, PidResult, RobotId,
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
        app.add_systems(Startup, setup_stabalize);
        app.add_systems(Update, stabalize_system);
    }
}

#[derive(Resource)]
struct StabilizeState {
    pitch: Entity,
    pitch_controller: PidController,

    roll: Entity,
    roll_controller: PidController,

    yaw: Entity,
    yaw_controller: PidController,
}

fn setup_stabalize(mut cmds: Commands, robot: Res<LocalRobot>) {
    let pitch = cmds
        .spawn((
            MovementContributionBundle {
                name: Name::new("Stabalize Pitch"),
                contribution: MovementContribution(Movement::default()),
                robot: RobotId(robot.net_id),
            },
            // TODO(high): Tune
            // TODO(low): Load from disk?
            PidConfig {
                kp: 0.3,
                ki: 0.15,
                kd: 0.1,
                max_integral: 40.0,
            },
            Replicate,
        ))
        .id();

    let roll = cmds
        .spawn((
            MovementContributionBundle {
                name: Name::new("Stabalize Roll"),
                contribution: MovementContribution(Movement::default()),
                robot: RobotId(robot.net_id),
            },
            // TODO(high): Tune
            // TODO(low): Load from disk?
            PidConfig {
                kp: 0.2,
                ki: 0.1,
                kd: 0.1,
                max_integral: 30.0,
            },
            Replicate,
        ))
        .id();

    let yaw = cmds
        .spawn((
            MovementContributionBundle {
                name: Name::new("Stabalize Yaw"),
                contribution: MovementContribution(Movement::default()),
                robot: RobotId(robot.net_id),
            },
            // TODO(high): Tune
            // TODO(low): Load from disk?
            PidConfig {
                kp: 0.2,
                ki: 0.1,
                kd: 0.15,
                max_integral: 30.0,
            },
            Replicate,
        ))
        .id();

    cmds.insert_resource(StabilizeState {
        pitch,
        pitch_controller: PidController::default(),
        roll,
        roll_controller: PidController::default(),
        yaw,
        yaw_controller: PidController::default(),
    });
}

fn stabalize_system(
    mut cmds: Commands,
    robot: Res<LocalRobot>,
    mut state: ResMut<StabilizeState>,
    robot_query: Query<(&Armed, &Orientation, &OrientationTarget)>,
    entity_query: Query<&PidConfig>,
    time: Res<Time<Real>>,
) {
    let robot = robot_query.get(robot.entity);
    let pitch_pid_config = entity_query.get(state.pitch).unwrap();
    let roll_pid_config = entity_query.get(state.roll).unwrap();
    let yaw_pid_config = entity_query.get(state.yaw).unwrap();

    if let Ok((&Armed::Armed, orientation, orientation_target)) = robot {
        let error = orientation_target.0 * orientation.0.inverse();

        //FIXME: Prefer roll over pitch
        let pitch_error = instant_twist(error, orientation.0 * Vec3A::X).to_degrees();
        let roll_error = instant_twist(error, orientation.0 * Vec3A::Y).to_degrees();
        let yaw_error = instant_twist(error, orientation.0 * Vec3A::Z).to_degrees();

        let res_pitch = state
            .pitch_controller
            .update(pitch_error, pitch_pid_config, time.delta());
        let res_roll = state
            .roll_controller
            .update(roll_error, roll_pid_config, time.delta());
        let res_yaw = state
            .yaw_controller
            .update(yaw_error, yaw_pid_config, time.delta());

        let pitch_movement = Movement {
            force: Vec3A::ZERO,
            torque: /*orientation.0.inverse() **/ Vec3A::X * res_pitch.correction,
        };

        let roll_movement = Movement {
            force: Vec3A::ZERO,
            torque: /*orientation.0.inverse() **/ Vec3A::Y * res_roll.correction,
        };

        let yaw_movement = Movement {
            force: Vec3A::ZERO,
            torque: /*orientation.0.inverse() **/ Vec3A::Z * res_yaw.correction,
        };

        cmds.entity(state.pitch)
            .insert((MovementContribution(pitch_movement), res_pitch));
        cmds.entity(state.roll)
            .insert((MovementContribution(roll_movement), res_roll));
        cmds.entity(state.yaw)
            .insert((MovementContribution(yaw_movement), res_yaw));
    } else {
        cmds.entity(state.pitch)
            .remove::<(MovementContribution, PidResult)>();
        cmds.entity(state.roll)
            .remove::<(MovementContribution, PidResult)>();
        cmds.entity(state.yaw)
            .remove::<(MovementContribution, PidResult)>();

        state.pitch_controller.reset_i();
        state.roll_controller.reset_i();
        state.yaw_controller.reset_i();
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
