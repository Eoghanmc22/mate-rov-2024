#![feature(test)]

// +X: Right, +Y: Forwards, +Z: Up
// +XR: Pitch Up, +YR: Roll Clockwise, +ZR: Yaw Counter Clockwise (top view)

pub mod blue_rov;
pub mod motor_preformance;
pub mod motor_relations;
pub mod solve;
pub mod utils;
pub mod x3d;

use std::hash::Hash;

use ahash::HashMap;
use glam::Vec3A;
use serde::{Deserialize, Serialize};

pub struct MotorConfig<MotorId: Hash + Eq> {
    motors: HashMap<MotorId, Motor>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Motor {
    /// Offset from origin
    pub position: Vec3A,
    /// Unit vector
    pub orientation: Vec3A,

    pub direction: Direction,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

impl Direction {
    pub fn get_sign(&self) -> f32 {
        match self {
            Direction::Clockwise => 1.0,
            Direction::CounterClockwise => -1.0,
        }
    }

    pub fn from_sign(value: f32) -> Direction {
        if value.signum() > 0.0 {
            Direction::Clockwise
        } else {
            Direction::CounterClockwise
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Movement {
    pub force: Vec3A,
    pub torque: Vec3A,
}
