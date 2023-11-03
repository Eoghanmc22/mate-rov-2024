use bevy::prelude::*;
use common::{components::RobotMarker, ecs_sync::NetworkId};

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.world.spawn((RobotMarker, NetworkId::SINGLETON));
    }
}
