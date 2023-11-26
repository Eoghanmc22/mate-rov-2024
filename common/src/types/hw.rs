use serde::{Deserialize, Serialize};

use super::units::{Celsius, Dps, GForce, Gauss, Mbar, Meters};

//
// Output
//

pub type PwmChannelId = u8;

//
// Input
//

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InertialFrame {
    pub gyro_x: Dps,
    pub gyro_y: Dps,
    pub gyro_z: Dps,

    pub accel_x: GForce,
    pub accel_y: GForce,
    pub accel_z: GForce,

    pub tempature: Celsius,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MagneticFrame {
    pub mag_x: Gauss,
    pub mag_y: Gauss,
    pub mag_z: Gauss,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DepthFrame {
    pub depth: Meters,
    pub altitude: Meters,
    pub pressure: Mbar,

    pub temperature: Celsius,
}
