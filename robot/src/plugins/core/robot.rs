use bevy::prelude::*;
use common::{
    bundles::RobotCoreBundle,
    components::{Robot, RobotId, RobotStatus, Singleton},
    ecs_sync::{NetId, Replicate},
    InstanceName,
};

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

fn setup_robot(mut cmds: Commands, name: Res<InstanceName>) {
    let net_id = NetId::random();

    let robot = cmds
        .spawn((
            RobotCoreBundle {
                name: Name::new(name.0.clone()),
                status: RobotStatus::default(),
                robot_id: RobotId(net_id),
                marker: Robot,
            },
            LocalRobotMarker,
            Replicate,
            Singleton,
            net_id,
        ))
        .id();

    cmds.insert_resource(LocalRobot {
        entity: robot,
        net_id,
    })
}
