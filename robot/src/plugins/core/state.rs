use bevy::prelude::*;
use common::components::{Armed, RobotMarker, RobotStatus};

use super::sync::Peer;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_state);
    }
}

pub fn update_state(
    mut cmds: Commands,
    peers: Query<&Peer>,
    robot: Query<(Entity, Option<&Armed>), With<RobotMarker>>,
) {
    let (robot, armed) = robot.single();
    let mut robot = cmds.entity(robot);

    if !peers.is_empty() {
        if let Some(Armed::Armed) = armed {
            robot.insert(RobotStatus::Ready);
            // TODO Moving state
        } else {
            robot.insert(RobotStatus::Disarmed);
        }
    } else {
        robot.insert(RobotStatus::NoPeer);
    }
}
