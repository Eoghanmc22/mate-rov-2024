use bevy::prelude::*;
use common::{
    components::{MovementContribution, RobotId, RobotMarker},
    ecs_sync::NetworkId,
};
use motor_math::Movement;

pub struct MotorMathPlugin;

impl Plugin for MotorMathPlugin {
    fn build(&self, app: &mut App) {
        // app.add_systems(Startup, start_hw_stat_thread);
        // app.add_systems(Update, (read_new_data, shutdown));
    }
}

// TODO: IO thread

// TODO: Sum all motor contributions

// TODO: Apply amperage correction

// TODO: Forces -> PWM commands -> Notify io thread

pub fn accumulate_movements(
    mut cmds: Commands,
    robot: Query<(Entity, &NetworkId), With<RobotMarker>>,
    movements: Query<(&RobotId, &MovementContribution)>,
) {
    let (entity, net_id) = robot.single();
    let robot = cmds.entity(entity);

    let mut total_movement = Movement::default();

    for (RobotId(robot_net_id), movement) in &movements {
        if robot_net_id == net_id {
            total_movement += movement.0;
        }
    }
}
