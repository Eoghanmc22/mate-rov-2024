use std::time::Duration;

use bevy::{
    app::App,
    reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize},
};
use serde::{Deserialize, Serialize};

use crate::components::{PidConfig, PidResult};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Reflect, Default)]
#[reflect(Serialize, Deserialize, Debug, Default)]
pub struct PidController {
    last_error: Option<f32>,
    integral: f32,
}

impl PidController {
    pub fn new() -> Self {
        Self {
            last_error: None,
            integral: 0.0,
        }
    }

    pub fn update(&mut self, error: f32, config: &PidConfig, interval: Duration) -> PidResult {
        let cfg = config;
        let interval = interval.as_secs_f32();

        self.integral += error * interval;
        self.integral = self.integral.clamp(-cfg.max_integral, cfg.max_integral);

        let proportional = error;
        let integral = self.integral;
        let derivative = (error - self.last_error.unwrap_or(error)) / interval;

        self.last_error = Some(error);

        let p = cfg.kp * proportional;
        let i = cfg.ki * integral;
        let d = cfg.kd * derivative;

        let correction = p + i + d;

        PidResult {
            p,
            i,
            d,
            correction,
        }
    }

    pub fn reset_i(&mut self) {
        self.integral = 0.0;
    }
}

pub fn register_types(app: &mut App) {
    app.register_type::<PidConfig>();
}
