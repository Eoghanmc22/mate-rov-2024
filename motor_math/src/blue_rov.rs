use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{utils::VectorTransform, Motor, MotorConfig};

/// Motor ids for blue rov heavy
#[derive(
    Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive,
)]
#[repr(u8)]
pub enum HeavyMotorId {
    LateralFrontLeft,
    LateralFrontRight,
    LateralBackLeft,
    LateralBackRight,
    VerticalFrontLeft,
    VerticalFrontRight,
    VerticalBackLeft,
    VerticalBackRight,
}

impl MotorConfig<HeavyMotorId> {
    pub fn new(lateral_front_right: Motor, vertical_front_right: Motor) -> Self {
        #[rustfmt::skip]
        let motors = [
            (HeavyMotorId::LateralFrontRight, lateral_front_right, &[].as_slice()),
            (HeavyMotorId::LateralFrontLeft, lateral_front_right, &[VectorTransform::ReflectYZ].as_slice()),
            (HeavyMotorId::LateralBackRight, lateral_front_right, &[VectorTransform::ReflectXZ].as_slice()),
            (HeavyMotorId::LateralBackLeft, lateral_front_right, &[VectorTransform::ReflectYZ, VectorTransform::ReflectXZ].as_slice()),

            (HeavyMotorId::VerticalFrontRight, vertical_front_right, &[].as_slice()),
            (HeavyMotorId::VerticalFrontLeft, vertical_front_right, &[VectorTransform::ReflectYZ].as_slice()),
            (HeavyMotorId::VerticalBackRight, vertical_front_right, &[VectorTransform::ReflectXZ].as_slice()),
            (HeavyMotorId::VerticalBackLeft, vertical_front_right, &[VectorTransform::ReflectYZ, VectorTransform::ReflectXZ].as_slice()),
        ];

        let motors = motors.into_iter().map(|(motor_id, seed, transforms)| {
            let (position, orientation) = transforms.iter().fold(
                (seed.position, seed.orientation),
                |(position, orientation), transform| {
                    (
                        transform.transform(position),
                        transform.transform(orientation),
                    )
                },
            );

            (
                motor_id,
                Motor {
                    position,
                    orientation,
                    direction: seed.direction.flip_n(transforms.len() as _),
                },
            )
        });

        Self::new_raw(motors)
    }
}
