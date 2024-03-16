use bevy::prelude::*;
use common::{
    components::{Armed, RobotStatus},
    sync::Peer,
};

use super::robot::LocalRobotMarker;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_state)
            .add_systems(Update, log_state_transition);
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

        // The robot should be disarmed when there are no peers controlling it
        if let Some(Armed::Armed) = armed {
            robot.insert(Armed::Disarmed);
        }
    }
}

fn log_state_transition(robot: Query<Ref<RobotStatus>, With<LocalRobotMarker>>) {
    for status in &robot {
        if status.is_changed() {
            info!("Robot State: {status:?}");
        }
    }
}
