//! Desired Movement -> Motor Commands

use std::hash::Hash;

use ahash::{HashMap, HashMapExt};
use nalgebra::Vector6;

use crate::{
    motor_preformance::{MotorData, MotorRecord},
    MotorConfig, Movement,
};

pub fn reverse_solve<MotorId: Hash + Ord + Clone>(
    movement: Movement,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
) -> HashMap<MotorId, MotorRecord> {
    let movement_vec = Vector6::from_iterator(
        [movement.force, movement.torque]
            .into_iter()
            .flat_map(|it| it.to_array().into_iter()),
    );

    let forces = motor_config.pseudo_inverse.clone() * movement_vec;

    let mut motor_cmds = HashMap::new();
    for (idx, (motor, _)) in motor_config.motors.iter().enumerate() {
        let force = forces[idx];

        let data = motor_data.lookup_by_force(force, true);

        motor_cmds.insert(motor.clone(), data);
    }

    motor_cmds
}

// TODO: Preserve force ratios
pub fn clamp_amperage<MotorId: Hash + Ord + Clone>(
    motor_cmds: HashMap<MotorId, MotorRecord>,
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
        let adjusted_current = data.current.copysign(data.force) * amperage_ratio;
        let data_adjusted = motor_data.lookup_by_current(adjusted_current, true);

        adjusted_motor_cmds.insert(motor_id.clone(), data_adjusted);
    }

    adjusted_motor_cmds
}
