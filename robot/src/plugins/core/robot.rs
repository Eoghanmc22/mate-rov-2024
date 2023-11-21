use bevy::prelude::*;
use common::{components::RobotMarker, ecs_sync::NetworkId};

pub struct RobotPlugin;

#[derive(Component, Debug, Copy, Clone, PartialEq, Default)]
pub struct LocalRobotMarker;

#[derive(Resource)]
pub struct LocalRobot(pub Entity);

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        let robot = app
            .world
            .spawn((RobotMarker, LocalRobotMarker, NetworkId::SINGLETON))
            .id();

        app.world.insert_resource(LocalRobot(robot))
    }
}
