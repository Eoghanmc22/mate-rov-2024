use serde::{Deserialize, Serialize};

use super::units::{Celsius, Dps, GForce, Gauss};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InertialFrame {
    pub gyro_x: Dps,
    pub gyro_y: Dps,
    pub gyro_z: Dps,

    pub accel_x: GForce,
    pub accel_y: GForce,
    pub accel_z: GForce,

    pub tempature: Celsius,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MagFrame {
    pub mag_x: Gauss,
    pub mag_y: Gauss,
    pub mag_z: Gauss,
}
