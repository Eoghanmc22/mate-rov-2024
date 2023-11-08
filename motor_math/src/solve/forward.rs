//! Motor Commands -> Movement

use std::hash::Hash;

use ahash::HashMap;

use crate::{motor_relations::MotorRelation, MotorConfig, Movement};

pub fn forward_solve<MotorId: Hash + Eq>(
    motor_config: &MotorConfig<MotorId>,
    motor_forces: &HashMap<MotorId, f32>,
) -> Movement {
    let mut movement = Movement::default();

    for (motor_id, &force) in motor_forces {
        // FIXME: Panics
        let motor = motor_config.motors[motor_id];
        let relation: MotorRelation = motor.into();

        movement.force += relation.force * force;
        movement.torque += relation.torque * force;
    }

    movement
}
