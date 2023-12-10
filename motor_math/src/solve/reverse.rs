//! Desired Movement -> Motor Commands

use std::fmt::Debug;
use std::hash::Hash;

use ahash::{HashMap, HashMapExt};
use nalgebra::Vector6;
use tracing::instrument;

use crate::{
    motor_preformance::{Interpolation, MotorData, MotorRecord},
    MotorConfig, Movement,
};

#[instrument(level = "trace", skip(motor_config), ret)]
pub fn reverse_solve<MotorId: Hash + Ord + Clone + Debug>(
    movement: Movement,
    motor_config: &MotorConfig<MotorId>,
) -> HashMap<MotorId, f32> {
    let movement_vec = Vector6::from_iterator(
        [movement.force, movement.torque]
            .into_iter()
            .flat_map(|it| it.to_array().into_iter()),
    );

    let forces = motor_config.pseudo_inverse.clone() * movement_vec;

    let mut motor_forces = HashMap::new();
    for (idx, (motor_id, _motor)) in motor_config.motors.iter().enumerate() {
        motor_forces.insert(motor_id.clone(), forces[idx]);
    }

    motor_forces
}

#[instrument(level = "trace", skip(motor_config, motor_data), ret)]
pub fn forces_to_cmds<MotorId: Hash + Ord + Clone + Debug>(
    forces: HashMap<MotorId, f32>,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
) -> HashMap<MotorId, MotorRecord> {
    let mut motor_cmds = HashMap::new();
    for (motor_id, force) in forces {
        // TODO/FIXME: Remove
        let motor = motor_config.motor(&motor_id).expect("Bad motor id");

        let data = motor_data.lookup_by_force(force, Interpolation::LerpDirection(motor.direction));

        motor_cmds.insert(motor_id.clone(), data);
    }

    motor_cmds
}

// TODO: Preserve force ratios
#[instrument(level = "trace", skip(motor_config, motor_data), ret)]
pub fn clamp_amperage<MotorId: Hash + Ord + Clone + Debug>(
    motor_cmds: HashMap<MotorId, MotorRecord>,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
    amperage_cap: f32,
) -> HashMap<MotorId, MotorRecord> {
    let mut amperage_total = 0.0;

    for data in motor_cmds.values() {
        amperage_total += data.current;
    }

    if amperage_total <= amperage_cap {
        return motor_cmds;
    } else {
        // TODO remove?
        println!("CURRENT LIMIT HIT");
    }

    let amperage_ratio = amperage_cap / amperage_total;

    let mut adjusted_motor_cmds = HashMap::default();
    for (motor_id, data) in motor_cmds {
        // FIXME: Fails silently
        let direction = motor_config
            .motor(&motor_id)
            .map(|it| it.direction)
            .unwrap_or(crate::Direction::Clockwise);

        let adjusted_current = data.current.copysign(data.force) * amperage_ratio;
        let data_adjusted =
            motor_data.lookup_by_current(adjusted_current, Interpolation::LerpDirection(direction));

        adjusted_motor_cmds.insert(motor_id.clone(), data_adjusted);
    }

    adjusted_motor_cmds
}
