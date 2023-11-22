#![feature(test)]

// +X: Right, +Y: Forwards, +Z: Up
// +XR: Pitch Up, +YR: Roll Clockwise, +ZR: Yaw Counter Clockwise (top view)

pub mod blue_rov;
pub mod motor_preformance;
pub mod solve;
pub mod utils;
pub mod x3d;

use std::{
    collections::BTreeMap,
    fmt::Debug,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use glam::Vec3A;
use nalgebra::{Matrix6xX, MatrixXx6};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MotorConfig<MotorId: Ord> {
    // TODO/FIXME: Is there any reason this isnt a Vec?
    motors: BTreeMap<MotorId, Motor>,

    matrix: Matrix6xX<f32>,
    pseudo_inverse: MatrixXx6<f32>,
}

impl<MotorId: Ord + Debug> MotorConfig<MotorId> {
    #[instrument(level = "trace", skip_all, ret)]
    pub fn new_raw(motors: impl IntoIterator<Item = (MotorId, Motor)>) -> Self {
        let motors: BTreeMap<_, _> = motors.into_iter().collect();

        let matrix = Matrix6xX::from_iterator(
            motors.len(),
            motors
                .iter()
                .flat_map(|it| {
                    [it.1.orientation, it.1.position.cross(it.1.orientation)].into_iter()
                })
                .flat_map(|it| it.to_array().into_iter()),
        );

        let pseudo_inverse = matrix.clone().pseudo_inverse(0.0001).unwrap();

        Self {
            motors,
            matrix,
            pseudo_inverse,
        }
    }

    pub fn motor(&self, motor: &MotorId) -> Option<&Motor> {
        self.motors.get(motor)
    }

    pub fn motors(&self) -> impl Iterator<Item = (&MotorId, &Motor)> {
        self.motors.iter()
    }
}

pub type ErasedMotorId = u8;

impl<MotorId: Ord + Into<ErasedMotorId> + Clone> MotorConfig<MotorId> {
    /// Order of ErasedMotorIds must match the order of MotorId given by the ord trait
    pub fn erase(self) -> MotorConfig<ErasedMotorId> {
        let MotorConfig {
            motors,
            matrix,
            pseudo_inverse,
        } = self;

        let motors = motors
            .into_iter()
            .map(|(id, motor)| (id.into(), motor))
            .collect();

        MotorConfig {
            motors,
            matrix,
            pseudo_inverse,
        }
    }
}

impl MotorConfig<ErasedMotorId> {
    /// Order of ErasedMotorIds must match the order of MotorId given by the ord trait
    pub fn unerase<MotorId: Ord + TryFrom<ErasedMotorId>>(
        self,
    ) -> Result<MotorConfig<MotorId>, <MotorId as TryFrom<ErasedMotorId>>::Error> {
        let MotorConfig {
            motors,
            matrix,
            pseudo_inverse,
        } = self;

        let motors = motors
            .into_iter()
            .map(|(id, motor)| MotorId::try_from(id).map(|it| (it, motor)))
            .collect::<Result<_, _>>()?;

        Ok(MotorConfig {
            motors,
            matrix,
            pseudo_inverse,
        })
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct Motor {
    /// Offset from origin
    pub position: Vec3A,
    /// Unit vector
    pub orientation: Vec3A,

    pub direction: Direction,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    pub fn from_sign(sign: f32) -> Self {
        if sign.signum() == 1.0 {
            Direction::Clockwise
        } else {
            Direction::CounterClockwise
        }
    }

    pub fn flip_n(&self, count: i32) -> Self {
        let sign = self.get_sign();
        let new_sign = sign * (-1.0f32).powi(count);
        Self::from_sign(new_sign)
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Movement {
    pub force: Vec3A,
    pub torque: Vec3A,
}

impl Add for Movement {
    type Output = Movement;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            force: self.force + rhs.force,
            torque: self.torque + rhs.torque,
        }
    }
}

impl AddAssign for Movement {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Movement {
    type Output = Movement;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            force: self.force - rhs.force,
            torque: self.torque - rhs.torque,
        }
    }
}

impl SubAssign for Movement {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Movement {
    type Output = Movement;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            force: self.force * rhs,
            torque: self.torque * rhs,
        }
    }
}

impl MulAssign<f32> for Movement {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for Movement {
    type Output = Movement;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            force: self.force / rhs,
            torque: self.torque / rhs,
        }
    }
}

impl DivAssign<f32> for Movement {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
