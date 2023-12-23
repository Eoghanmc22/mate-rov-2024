use bevy::prelude::*;
use common::{
    components::{Armed, RobotStatus},
    sync::Peer,
};

use super::robot::LocalRobotMarker;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_state);
    }
}

// TODO(high): More nuanced state to drive the neopixels
fn update_state(
    mut cmds: Commands,
    peers: Query<&Peer>,
    robot: Query<(Entity, Option<&Armed>), With<LocalRobotMarker>>,
) {
    let (robot, armed) = robot.single();
    let mut robot = cmds.entity(robot);

    if !peers.is_empty() {
        if let Some(Armed::Armed) = armed {
            robot.insert(RobotStatus::Ready);
            // TODO(mid): Moving state
        } else {
            robot.insert(RobotStatus::Disarmed);
        }
    } else {
        robot.insert(RobotStatus::NoPeer);
    }
}
