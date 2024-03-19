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
    robot: Query<(Entity, Option<&RobotStatus>, Option<&Armed>), With<LocalRobotMarker>>,
) {
    let (robot, status, armed) = robot.single();
    let mut robot = cmds.entity(robot);

    if !peers.is_empty() {
        match armed {
            Some(Armed::Armed) => {
                if status != Some(&RobotStatus::Armed) {
                    robot.insert(RobotStatus::Armed);
                }
            }
            _ => {
                if status != Some(&RobotStatus::Disarmed) {
                    robot.insert(RobotStatus::Disarmed);
                }
            }
        }
    } else {
        if status != Some(&RobotStatus::NoPeer) {
            robot.insert(RobotStatus::NoPeer);
        }

        // The robot should be disarmed when there are no peers controlling it
        if let Some(Armed::Armed) = armed {
            robot.insert(Armed::Disarmed);
        }
    }
}

fn log_state_transition(robot: Query<Ref<RobotStatus>, With<LocalRobotMarker>>) {
    for status in &robot {
        if status.is_changed() {
            info!("Robot State: {:?}", *status);
        }
    }
}
