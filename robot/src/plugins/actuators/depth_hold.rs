use bevy::prelude::*;
use common::{
    bundles::MovementContributionBundle,
    components::{
        Armed, Depth, DepthTarget, MovementContribution, Orientation, PidConfig, PidResult, RobotId,
    },
    ecs_sync::Replicate,
    types::{units::Meters, utils::PidController},
};
use glam::Vec3A;
use motor_math::Movement;

use crate::plugins::core::robot::LocalRobot;

pub struct DepthHoldPlugin;

impl Plugin for DepthHoldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_depth_hold)
            .add_systems(Update, depth_hold_system);
    }
}

#[derive(Resource)]
struct DepthHoldState(Entity, PidController);

fn setup_depth_hold(mut cmds: Commands, robot: Res<LocalRobot>) {
    let entity = cmds
        .spawn((
            MovementContributionBundle {
                name: Name::new("Depth Hold"),
                contribution: MovementContribution(Movement::default()),
                robot: RobotId(robot.net_id),
            },
            // TODO(high): Tune
            // TODO(low): Load from disk?
            PidConfig {
                kp: 100.0,
                ki: 5.0,
                kd: 1.5,
                kt: 5000.0,
                max_integral: 10.0,
            },
            Replicate,
        ))
        .id();

    cmds.insert_resource(DepthHoldState(entity, PidController::default()));
}

fn depth_hold_system(
    mut last_target: Local<Option<Meters>>,
    mut cmds: Commands,
    robot: Res<LocalRobot>,
    mut state: ResMut<DepthHoldState>,
    robot_query: Query<(&Armed, &Depth, &DepthTarget, &Orientation)>,
    entity_query: Query<&PidConfig>,
    time: Res<Time<Real>>,
) {
    let robot = robot_query.get(robot.entity);
    let pid_config = entity_query.get(state.0).unwrap();

    if let Ok((&Armed::Armed, depth, depth_target, orientation)) = robot {
        let depth_error = depth_target.0 - depth.0.depth;
        let depth_td = depth_target.0 - last_target.unwrap_or(depth_target.0);

        let pid = &mut state.1;
        // Depth increases as Z decreases, flip the sign
        let res = pid.update(-depth_error.0, -depth_td.0, pid_config, time.delta());

        let correction = orientation.0.inverse() * Vec3A::Z * res.correction;
        let movement = Movement {
            force: correction,
            torque: Vec3A::ZERO,
        };

        cmds.entity(state.0)
            .insert((MovementContribution(movement), res));
        *last_target = Some(depth_target.0);
    } else {
        cmds.entity(state.0)
            .remove::<(MovementContribution, PidResult)>();
        *last_target = None;
    }
}
