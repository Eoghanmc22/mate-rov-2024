use ahash::{HashMap, HashSet};
use bevy::{ecs::system::Resource, transform::components::Transform};
use common::types::hw::PwmChannelId;
use glam::{vec3, EulerRot, Quat, Vec3A};
use motor_math::{blue_rov::HeavyMotorId, x3d::X3dMotorId, ErasedMotorId, Motor, MotorConfig};
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct RobotConfig {
    pub name: String,
    pub port: u16,

    pub motor_config: MotorConfigDefinition,
    pub servo_config: ServoConfigDefinition,

    pub motor_amperage_budget: f32,
    pub jerk_limit: f32,
    pub center_of_mass: Vec3A,

    pub cameras: HashMap<String, CameraDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MotorConfigDefinition {
    X3d(X3dDefinition),
    BlueRov(BlueRovDefinition),
    Custom(CustomDefinition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3dDefinition {
    pub seed_motor: Motor,

    pub motors: HashMap<X3dMotorId, PwmChannelId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueRovDefinition {
    pub vertical_seed_motor: Motor,
    pub lateral_seed_motor: Motor,

    pub motors: HashMap<HeavyMotorId, PwmChannelId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDefinition {
    pub motors: HashMap<String, CustomMotor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMotor {
    pub pwm_channel: PwmChannelId,
    pub motor: Motor,
}

impl X3dDefinition {
    fn to_motor_config(&self, center_mass: Vec3A) -> MotorConfig<X3dMotorId> {
        MotorConfig::<X3dMotorId>::new(self.seed_motor, center_mass)
    }
}

impl BlueRovDefinition {
    fn to_motor_config(&self, center_mass: Vec3A) -> MotorConfig<HeavyMotorId> {
        MotorConfig::<HeavyMotorId>::new(
            self.lateral_seed_motor,
            self.vertical_seed_motor,
            center_mass,
        )
    }
}

impl CustomDefinition {
    fn to_motor_config(&self, center_mass: Vec3A) -> MotorConfig<String> {
        MotorConfig::<String>::new_raw(
            self.motors
                .iter()
                .map(|(id, motor)| (id.to_owned(), motor.motor)),
            center_mass,
        )
    }
}

impl MotorConfigDefinition {
    // TODO(low): Rename and make less bad
    pub fn flatten(
        &self,
        center_mass: Vec3A,
    ) -> (
        impl Iterator<Item = (ErasedMotorId, Motor, PwmChannelId)>,
        MotorConfig<ErasedMotorId>,
    ) {
        let motors: Vec<_>;

        let config = match self {
            MotorConfigDefinition::X3d(x3d) => {
                let config: MotorConfig<_> = x3d.to_motor_config(center_mass);

                motors = config
                    .motors()
                    .map(|(id, motor)| {
                        (
                            (*id).into(),
                            *motor,
                            x3d.motors
                                .get(id)
                                .copied()
                                .expect("Incomplete motor definition"),
                        )
                    })
                    .collect();

                config.erase()
            }
            MotorConfigDefinition::BlueRov(blue_rov) => {
                let config: MotorConfig<_> = blue_rov.to_motor_config(center_mass);

                motors = config
                    .motors()
                    .map(|(id, motor)| {
                        (
                            (*id).into(),
                            *motor,
                            blue_rov
                                .motors
                                .get(id)
                                .copied()
                                .expect("Incomplete motor definition"),
                        )
                    })
                    .collect();

                config.erase()
            }
            MotorConfigDefinition::Custom(custom) => {
                let config: MotorConfig<_> = custom.to_motor_config(center_mass);

                motors = config
                    .motors()
                    .enumerate()
                    .map(|(idx, (id, motor))| {
                        (
                            idx as u8,
                            *motor,
                            custom
                                .motors
                                .get(id)
                                .map(|it| it.pwm_channel)
                                .expect("Incomplete motor definition"),
                        )
                    })
                    .collect();

                MotorConfig::new_raw(
                    config
                        .motors()
                        .enumerate()
                        .map(|(idx, (_, motor))| (idx as _, *motor)),
                    center_mass,
                )
            }
        };

        (motors.into_iter(), config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServoConfigDefinition {
    pub servos: HashMap<String, Servo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Servo {
    pub pwm_channel: PwmChannelId,
    pub cameras: HashSet<String>,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct CameraDefinition {
    pub name: String,
    pub transform: ConfigTransform,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTransform {
    position: ConfigPosition,
    rotation: ConfigRotation,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPosition {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRotation {
    yaw: f32,
    pitch: f32,
    roll: f32,
}

impl ConfigTransform {
    pub fn flatten(&self) -> Transform {
        let ConfigPosition { x, y, z } = self.position;
        let ConfigRotation { yaw, pitch, roll } = self.rotation;

        Transform::from_translation(Quat::from_rotation_x(90f32.to_radians()) * vec3(x, -y, z))
            .with_rotation(Quat::from_euler(
                EulerRot::default(),
                yaw.to_radians(),
                pitch.to_radians(),
                roll.to_radians(),
            ))
    }
}
