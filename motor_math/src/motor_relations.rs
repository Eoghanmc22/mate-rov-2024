use glam::Vec3;

use crate::Motor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotorRelation {
    pub force: Vec3,
    pub torque: Vec3,
    pub torque_norm: Vec3,
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
