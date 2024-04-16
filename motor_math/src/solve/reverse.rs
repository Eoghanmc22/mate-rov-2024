//! Desired Movement -> Motor Commands

use std::fmt::Debug;
use std::hash::Hash;

use ahash::{HashMap, HashMapExt};
use glam::vec3a;
use nalgebra::Vector6;
use serde::{Deserialize, Serialize};
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
        let motor = motor_config.motor(&motor_id).expect("Bad motor id");
        let data = motor_data.lookup_by_force(force, Interpolation::LerpDirection(motor.direction));

        motor_cmds.insert(motor_id.clone(), data);
    }

    motor_cmds
}

/// Does not preserve force ratios
/// Runs in constant time
#[instrument(level = "trace", skip(motor_config, motor_data), ret)]
pub fn clamp_amperage_fast<MotorId: Hash + Ord + Clone + Debug>(
    motor_cmds: HashMap<MotorId, MotorRecord>,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
    amperage_cap: f32,
) -> HashMap<MotorId, MotorRecord> {
    let amperage_total = motor_cmds.values().map(|it| it.current).sum::<f32>();

    if amperage_total <= amperage_cap {
        return motor_cmds;
    } else {
        // TODO remove?
        // println!("CURRENT LIMIT HIT");
    }

    let amperage_ratio = amperage_cap / amperage_total;

    let mut adjusted_motor_cmds = HashMap::default();
    for (motor_id, data) in motor_cmds {
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

#[instrument(level = "trace", skip(motor_config, motor_data), ret)]
pub fn clamp_amperage<MotorId: Hash + Ord + Clone + Debug>(
    motor_cmds: HashMap<MotorId, MotorRecord>,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
    amperage_cap: f32,
    epsilon: f32,
) -> HashMap<MotorId, MotorRecord> {
    let amperage_total = motor_cmds.values().map(|it| it.current).sum::<f32>();

    if amperage_total <= amperage_cap {
        return motor_cmds;
    } else {
        // TODO remove?
        // println!("CURRENT LIMIT HIT");
    }

    let force_ratio =
        binary_search_force_ratio(&motor_cmds, motor_config, motor_data, amperage_cap, epsilon);

    let mut adjusted_motor_cmds = HashMap::default();
    for (motor_id, data) in motor_cmds {
        let direction = motor_config
            .motor(&motor_id)
            .map(|it| it.direction)
            .unwrap_or(crate::Direction::Clockwise);

        let force_current = data.force * force_ratio;
        let data_adjusted =
            motor_data.lookup_by_force(force_current, Interpolation::LerpDirection(direction));

        adjusted_motor_cmds.insert(motor_id.clone(), data_adjusted);
    }

    adjusted_motor_cmds
}

pub fn binary_search_force_ratio<MotorId: Hash + Ord + Clone + Debug>(
    motor_cmds: &HashMap<MotorId, MotorRecord>,
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
    amperage_cap: f32,
    epsilon: f32,
) -> f32 {
    let (mut lower_bound, mut lower_current) = (0.0, 0.0);
    let (mut upper_bound, mut upper_current) = (f32::INFINITY, f32::INFINITY);
    let mut mid = 1.0;

    loop {
        let mid_current = motor_cmds
            .iter()
            .map(|(motor_id, data)| {
                let direction = motor_config
                    .motor(motor_id)
                    .map(|it| it.direction)
                    .unwrap_or(crate::Direction::Clockwise);

                let adjusted_force = data.force.copysign(data.force) * mid;
                let data = motor_data
                    .lookup_by_force(adjusted_force, Interpolation::LerpDirection(direction));

                data.current
            })
            .sum::<f32>();

        if (mid_current - amperage_cap).abs() < epsilon {
            return mid;
        }

        if mid_current >= amperage_cap {
            upper_bound = mid;
            upper_current = mid_current;
        } else {
            lower_bound = mid;
            lower_current = mid_current;
        }

        if upper_bound == f32::INFINITY {
            mid *= amperage_cap / mid_current;
            // mid *= 2.0;
        } else {
            let alpha = (amperage_cap - lower_current) / (upper_current - lower_current);
            mid = upper_bound * alpha + lower_bound * (1.0 - alpha)
            // mid = upper_bound / 2.0 + lower_bound / 2.0
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum Axis {
    X,
    Y,
    Z,
    XRot,
    YRot,
    ZRot,
}

impl Axis {
    pub fn movement(&self) -> Movement {
        match self {
            Axis::X => Movement {
                force: vec3a(1.0, 0.0, 0.0),
                torque: vec3a(0.0, 0.0, 0.0),
            },
            Axis::Y => Movement {
                force: vec3a(0.0, 1.0, 0.0),
                torque: vec3a(0.0, 0.0, 0.0),
            },
            Axis::Z => Movement {
                force: vec3a(0.0, 0.0, 1.0),
                torque: vec3a(0.0, 0.0, 0.0),
            },
            Axis::XRot => Movement {
                force: vec3a(0.0, 0.0, 0.0),
                torque: vec3a(1.0, 0.0, 0.0),
            },
            Axis::YRot => Movement {
                force: vec3a(0.0, 0.0, 0.0),
                torque: vec3a(0.0, 1.0, 0.0),
            },
            Axis::ZRot => Movement {
                force: vec3a(0.0, 0.0, 0.0),
                torque: vec3a(0.0, 0.0, 1.0),
            },
        }
    }
}

pub fn axis_maximums<MotorId: Hash + Ord + Clone + Debug>(
    motor_config: &MotorConfig<MotorId>,
    motor_data: &MotorData,
    amperage_cap: f32,
    epsilon: f32,
) -> HashMap<Axis, f32> {
    [
        Axis::X,
        Axis::Y,
        Axis::Z,
        Axis::XRot,
        Axis::YRot,
        Axis::ZRot,
    ]
    .into_iter()
    .map(|it| (it, it.movement()))
    .map(|(axis, movement)| {
        let initial = 25.0;

        let forces = reverse_solve(movement * initial, motor_config);
        let cmds = forces_to_cmds(forces, motor_config, motor_data);
        let scale =
            binary_search_force_ratio(&cmds, motor_config, motor_data, amperage_cap, epsilon);

        let value = scale * initial;
        (axis, value)
    })
    .collect()
}
