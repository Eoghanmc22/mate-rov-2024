use std::time::Duration;

use ahash::HashMap;
use bevy::prelude::*;
use common::{
    components::{
        ActualForce, ActualMovement, CurrentDraw, MotorContribution, MotorDefinition, Motors,
        MovementContribution, MovementCurrentCap, PwmSignal, RobotId, TargetForce, TargetMovement,
    },
    ecs_sync::NetId,
};
use motor_math::{
    motor_preformance::{self, Interpolation, MotorData},
    solve, Direction, Movement,
};

use crate::plugins::core::robot::LocalRobotMarker;

pub struct MotorMathPlugin;

impl Plugin for MotorMathPlugin {
    fn build(&self, app: &mut App) {
        // TODO/FIXME: This is kinda bad
        let motor_data =
            motor_preformance::read_motor_data("motor_data.csv").expect("Read motor data");

        app.add_systems(
            Update,
            (
                accumulate_movements,
                accumulate_motor_forces.after(accumulate_movements),
            ),
        );
        app.insert_resource(MotorDataRes(motor_data));
    }
}

#[derive(Resource)]
struct MotorDataRes(MotorData);

pub fn accumulate_movements(
    mut cmds: Commands,
    robot: Query<(Entity, &NetId, &Motors), With<LocalRobotMarker>>,
    movements: Query<(&RobotId, &MovementContribution)>,

    motor_data: Res<MotorDataRes>,
) {
    let (entity, net_id, Motors(motor_config)) = robot.single();
    let mut robot = cmds.entity(entity);

    let mut total_movement = Movement::default();

    for (RobotId(robot_net_id), movement) in &movements {
        if robot_net_id == net_id {
            total_movement += movement.0;
        }
    }

    let forces = solve::reverse::reverse_solve(total_movement, motor_config);
    let motor_cmds = solve::reverse::forces_to_cmds(forces, motor_config, &motor_data.0);
    let forces = motor_cmds
        .into_iter()
        .map(|(motor, cmd)| (motor, cmd.force.into()))
        .collect();

    robot.insert(MotorContribution(forces));
}

// TODO: Split into smaller systems
pub fn accumulate_motor_forces(
    mut cmds: Commands,
    robot: Query<(Entity, &NetId, &Motors, &MovementCurrentCap), With<LocalRobotMarker>>,
    motor_forces: Query<(&RobotId, &MotorContribution)>,
    motors: Query<(Entity, &MotorDefinition, &RobotId)>,

    motor_data: Res<MotorDataRes>,
) {
    let (entity, net_id, Motors(motor_config), MovementCurrentCap(current_cap)) = robot.single();
    let mut robot = cmds.entity(entity);

    let mut all_forces = HashMap::default();

    for (RobotId(robot_net_id), motor_force_contributions) in &motor_forces {
        if robot_net_id == net_id {
            for (motor, force) in &motor_force_contributions.0 {
                *all_forces.entry(*motor).or_default() += force.0;
            }
        }
    }

    let target_movement = solve::forward::forward_solve(motor_config, &all_forces);
    robot.insert(TargetMovement(target_movement));

    let motor_cmds = all_forces
        .iter()
        .map(|(motor, force)| {
            // FIXME: Fails silently
            let direction = motor_config
                .motor(motor)
                .map(|it| it.direction)
                .unwrap_or(Direction::Clockwise);

            (
                *motor,
                motor_data
                    .0
                    .lookup_by_force(*force, Interpolation::LerpDirection(direction)),
            )
        })
        .collect();

    let motor_cmds =
        solve::reverse::clamp_amperage(motor_cmds, motor_config, &motor_data.0, current_cap.0);

    let motor_forces = motor_cmds
        .iter()
        .map(|(motor, data)| (*motor, data.force))
        .collect();

    let actual_movement = solve::forward::forward_solve(motor_config, &motor_forces);
    robot.insert(ActualMovement(actual_movement));

    for (motor_entity, MotorDefinition(id, _motor), RobotId(robot_net_id)) in &motors {
        if robot_net_id == net_id {
            let mut motor = cmds.entity(motor_entity);

            // TODO/FIXME: panics
            let target_force = all_forces.get(id);
            let actual_data = motor_cmds.get(id);

            if let (Some(target_force), Some(actual_data)) = (target_force, actual_data) {
                motor.insert((
                    TargetForce((*target_force).into()),
                    ActualForce(actual_data.force.into()),
                    CurrentDraw(actual_data.current.into()),
                    PwmSignal(Duration::from_micros(actual_data.pwm as u64)),
                ));
            } else {
                motor.insert((
                    TargetForce(0.0.into()),
                    ActualForce(0.0.into()),
                    CurrentDraw(0.0.into()),
                    PwmSignal(Duration::from_micros(1500)),
                ));
            }
        }
    }
}
