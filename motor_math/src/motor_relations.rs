use glam::Vec3A;

use crate::Motor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotorRelation {
    pub force: Vec3A,
    pub torque: Vec3A,
    pub torque_norm: Vec3A,
}

impl From<Motor> for MotorRelation {
    fn from(motor: Motor) -> Self {
        let force = motor.orientation;
        let torque = motor.position.cross(motor.orientation);

        Self {
            force,
            torque,
            torque_norm: torque.normalize(),
        }
    }
}
