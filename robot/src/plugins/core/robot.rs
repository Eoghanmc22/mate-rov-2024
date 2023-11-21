use bevy::prelude::*;
use common::{components::RobotMarker, ecs_sync::NetworkId};

pub struct RobotPlugin;

#[derive(Component, Debug, Copy, Clone, PartialEq, Default)]
pub struct LocalRobotMarker;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.world
            .spawn((RobotMarker, LocalRobotMarker, NetworkId::SINGLETON));
    }
}
