use crate::{utils::VectorTransform, Direction, Motor, MotorConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum X3dMotorId {
    FrontLeftBottom,
    FrontLeftTop,
    FrontRightBottom,
    FrontRightTop,
    BackLeftBottom,
    BackLeftTop,
    BackRightBottom,
    BackRightTop,
}

impl MotorConfig<X3dMotorId> {
    pub fn new(front_right_top: Motor) -> Self {
        #[rustfmt::skip]
        let motors = [
            (X3dMotorId::FrontRightTop, [].as_slice()),

            (X3dMotorId::FrontRightBottom, [VectorTransform::ReflectXY].as_slice()),
            (X3dMotorId::FrontLeftTop, [VectorTransform::ReflectYZ].as_slice()),
            (X3dMotorId::BackRightTop, [VectorTransform::ReflectXZ].as_slice()),

            (X3dMotorId::FrontLeftBottom, [VectorTransform::ReflectXY, VectorTransform::ReflectYZ].as_slice()),
            (X3dMotorId::BackLeftTop, [VectorTransform::ReflectYZ, VectorTransform::ReflectXZ].as_slice()),
            (X3dMotorId::BackRightBottom, [VectorTransform::ReflectXZ, VectorTransform::ReflectXY].as_slice()),

            (X3dMotorId::BackLeftBottom, [VectorTransform::ReflectXY, VectorTransform::ReflectYZ, VectorTransform::ReflectXZ].as_slice()),
        ];

        let motors = motors
            .into_iter()
            .map(|(motor_id, transforms)| {
                let (position, orientation) = transforms.iter().fold(
                    (front_right_top.position, front_right_top.orientation),
                    |(position, orientation), transform| {
                        (
                            transform.transform(position),
                            transform.transform(orientation),
                        )
                    },
                );

                // TODO/FIXME
                // let direction = Direction::from_sign(
                //     front_right_top.direction.get_sign() * (-1.0f32).powi(transforms.len() as _),
                // );
                let direction = Direction::Clockwise;

                (
                    motor_id,
                    Motor {
                        position,
                        orientation,
                        direction,
                    },
                )
            })
            .collect();

        Self { motors }
    }
}
