//! Motor Commands -> Movement

use ahash::HashMap;
use glam::Vec3A;
use nalgebra::DVector;
use std::{fmt::Debug, hash::Hash};
use tracing::instrument;

use crate::{MotorConfig, Movement};

#[instrument(level = "trace", skip(motor_config), ret)]
pub fn forward_solve<MotorId: Hash + Ord + Debug>(
    motor_config: &MotorConfig<MotorId>,
    motor_forces: &HashMap<MotorId, f32>,
) -> Movement {
    let force_vec = DVector::from_iterator(
        motor_config.motors.len(),
        motor_config
            .motors
            .keys()
            .map(|id| motor_forces.get(id).copied().unwrap_or(0.0)),
    );

    let movement = motor_config.matrix.clone() * force_vec;
    let movement = movement.as_slice();

    Movement {
        force: Vec3A::from_slice(&movement[0..3]),
        torque: Vec3A::from_slice(&movement[3..6]),
    }
}
