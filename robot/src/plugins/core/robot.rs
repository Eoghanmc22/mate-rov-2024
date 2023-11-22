use bevy::prelude::*;
use common::{
    bundles::RobotCoreBundle,
    components::{RobotId, RobotMarker, RobotStatus},
    ecs_sync::NetworkId,
};

use crate::config::RobotConfig;

pub struct RobotPlugin;

#[derive(Component, Debug, Copy, Clone, PartialEq, Default)]
pub struct LocalRobotMarker;

#[derive(Resource)]
pub struct LocalRobot(pub Entity);

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        let robot_config: &RobotConfig = app.world.resource();

        let robot = app
            .world
            .spawn((
                RobotCoreBundle {
                    status: RobotStatus::default(),
                    net_id: NetworkId::SINGLETON,
                    robot_id: RobotId(NetworkId::SINGLETON),
                    marker: RobotMarker(robot_config.name.clone()),
                },
                LocalRobotMarker,
            ))
            .id();

        app.world.insert_resource(LocalRobot(robot))
    }
}
