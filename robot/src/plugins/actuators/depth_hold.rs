use bevy::prelude::*;
use common::{
    bundles::MovementContributionBundle,
    components::{
        ActuatorContributionMarker, Depth, DepthTarget, MovementContribution, Orientation,
        PidConfig, PidResult, RobotId,
    },
    ecs_sync::Replicate,
    types::utils::PidController,
};
use glam::Vec3A;
use motor_math::Movement;

use crate::plugins::core::robot::LocalRobot;

pub struct DepthHoldPlugin;

impl Plugin for DepthHoldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, depth_hold_system);

        let robot_net_id = app.world.resource::<LocalRobot>().net_id;

        let entity = app
            .world
            .spawn((
                Name::new("Depth Hold"),
                MovementContributionBundle {
                    marker: ActuatorContributionMarker("Depth Hold".to_owned()),
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
        app.insert_resource(DepthHoldState(entity, PidController::default()));
    }
}

#[derive(Resource)]
struct DepthHoldState(Entity, PidController);

pub fn depth_hold_system(
    mut cmds: Commands,
    robot: Res<LocalRobot>,
    mut state: ResMut<DepthHoldState>,
    robot_query: Query<(&Depth, &DepthTarget, &Orientation)>,
    entity_query: Query<&PidConfig>,
    time: Res<Time<Real>>,
) {
    let robot = robot_query.get(robot.entity);
    let pid_config = entity_query.get(state.0).unwrap();

    if let Ok((depth, depth_target, orientation)) = robot {
        let depth_error = depth_target.0 - depth.0.depth;

        let pid = &mut state.1;
        // Depth increases as Z decreases, flip the sign
        let res = pid.update(-depth_error.0, pid_config, time.delta());

        let correction = orientation.0.inverse() * Vec3A::Z * res.correction;
        let movement = Movement {
            force: correction,
            torque: Vec3A::ZERO,
        };

        cmds.entity(state.0)
            .insert((MovementContribution(movement), res));
    } else {
        cmds.entity(state.0)
            .remove::<(MovementContribution, PidResult)>();
    }
}
