//! Desired Movement -> Motor Commands

use std::{fmt::Debug, hash::Hash};

use ahash::HashMap;
use glam::{vec3, Vec3};

use crate::{
    motor_preformance::{MotorData, MotorRecord},
    motor_relations::MotorRelation,
    MotorConfig, Movement,
};

// FIXME: This needs to know about CW/CCW
// TODO: Use iterators?
pub fn reverse_solve<MotorId: Hash + Eq + Clone + Debug>(
    movement: Movement,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
    amperage_cap: f32,
) -> HashMap<MotorId, MotorRecord> {
    let mut motor_contributions = motor_config
        .motors
        .iter()
        // (motor_id, (relation, force, torque, force_correction, torque_correction))
        .map(|(id, motor)| {
            let relation = MotorRelation::from(*motor);

            (
                id,
                (
                    relation,
                    Vec3::default(),
                    Vec3::default(),
                    Vec3::default(),
                    Vec3::default(),
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    // Iterative force solver
    for _ in 0..100 {
        let force_total = motor_contributions
            .values()
            .map(|(_, force, _, _, _)| *force)
            .sum::<Vec3>();
        let torque_total = motor_contributions
            .values()
            .map(|(_, _, torque, _, _)| *torque)
            .sum::<Vec3>();

        println!("force_total: {force_total:.5}, torque_total: {torque_total:.5}");

        let force_error = movement.force - force_total;
        let torque_error = movement.torque - torque_total;

        println!("force_error: {force_error:.5}, torque_error: {torque_error:.5}");

        for (relation, force, torque, force_correction, torque_correction) in
            motor_contributions.values_mut()
        {
            *force_correction = force_error.project_onto_normalized(relation.force);
            *torque_correction = torque_error.project_onto_normalized(relation.torque_norm);
        }

        let force_correction_total = motor_contributions
            .values()
            .map(|(_, _, _, force_correction, _)| *force_correction)
            .sum::<Vec3>();
        let torque_correction_total = motor_contributions
            .values()
            .map(|(_, _, _, _, torque_correction)| *torque_correction)
            .sum::<Vec3>();

        println!("force_correction_total: {force_correction_total:.5}, torque_correction_total: {torque_correction_total:.5}");

        // todo check
        if force_correction_total != Vec3::ZERO {
            let force_correction_total_norm = force_correction_total.normalize_or_zero();
            let force_correction_scale =
                force_error.dot(force_correction_total_norm) / force_correction_total.length();

            println!("force_correction_scale: {force_correction_scale:.5}");

            for (relation, force, torque, force_correction, torque_correction) in
                motor_contributions.values_mut()
            {
                *force += *force_correction * force_correction_scale;

                println!("force: {force:.5}");
            }
        }

        // todo check
        if torque_correction_total != Vec3::ZERO {
            let torque_correction_total_norm = torque_correction_total.normalize_or_zero();
            let torque_correction_scale =
                torque_error.dot(torque_correction_total_norm) / torque_correction_total.length();

            println!("torque_correction_total: {torque_correction_total:.5}");

            for (relation, force, torque, force_correction, torque_correction) in
                motor_contributions.values_mut()
            {
                *torque += *torque_correction * torque_correction_scale;

                println!("torque: {torque:.5}");
            }
        }

        println!();
        println!();

        if force_correction_total == Vec3::ZERO && torque_correction_total == Vec3::ZERO {
            break;
        }
    }

    let mut motor_cmds = HashMap::default();
    let mut amperage_total = 0.0;
    for (motor_id, (relation, force, torque, _, _)) in motor_contributions {
        // TODO: Is this correct
        let motor_force = force.dot(relation.force) / relation.force.length_squared()
            + torque.dot(relation.torque) / relation.torque.length_squared();
        let data = motor_data.lookup_by_force(motor_force, true);

        amperage_total += data.current;

        motor_cmds.insert(motor_id.clone(), data);
    }

    if amperage_cap >= amperage_total {
        return motor_cmds;
    } else {
        println!("CURRENT LIMIT HIT");
    }

    let amperage_ratio = if amperage_total != 0.0 {
        amperage_cap / amperage_total
    } else {
        0.0
    };

    let mut adjusted_motor_cmds = HashMap::default();
    for (motor_id, data) in motor_cmds {
        let adjusted_current = data.current.copysign(data.force) * amperage_ratio;
        let data_adjusted = motor_data.lookup_by_current(adjusted_current, true);

        adjusted_motor_cmds.insert(motor_id.clone(), data_adjusted);
    }

    adjusted_motor_cmds
}

pub fn maximium_forces() {
    // TODO
}

// fn recip_or_zero(vec: Vec3) -> Vec3 {
//     let x = vec.x.recip();
//     let y = vec.y.recip();
//     let z = vec.z.recip();
//
//     vec3(
//         if x.is_finite() { x } else { 0.0 },
//         if y.is_finite() { y } else { 0.0 },
//         if z.is_finite() { z } else { 0.0 },
//     )
// }

fn recip_or_zero(val: f32) -> f32 {
    let val = val.recip();

    if val.is_finite() {
        val
    } else {
        0.0
    }
}
