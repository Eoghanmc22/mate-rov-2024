use bevy::prelude::*;
use common::{
    bundles::RobotCoreBundle,
    components::{RobotId, RobotMarker, RobotStatus},
    ecs_sync::{NetId, Replicate},
};

use crate::config::RobotConfig;

pub struct RobotPlugin;

#[derive(Component, Debug, Copy, Clone, PartialEq, Default)]
pub struct LocalRobotMarker;

#[derive(Resource)]
pub struct LocalRobot {
    pub entity: Entity,
    pub net_id: NetId,
}

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup_robot);
    }
}

fn setup_robot(mut cmds: Commands, config: Res<RobotConfig>) {
    let net_id = NetId::random();

    let robot = cmds
        .spawn((
            Name::new(format!("{} ROV", config.name)),
            RobotCoreBundle {
                status: RobotStatus::default(),
                robot_id: RobotId(net_id),
                marker: RobotMarker(config.name.clone()),
            },
            LocalRobotMarker,
            Replicate,
            net_id,
        ))
        .id();

    cmds.insert_resource(LocalRobot {
        entity: robot,
        net_id,
    })
}
