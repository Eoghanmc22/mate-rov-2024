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

    last_deltas: [f32; 5],
    delta_idx: usize,
}

impl PidController {
    pub fn new() -> Self {
        Self {
            last_error: None,
            integral: 0.0,
            last_deltas: [0.0; 5],
            delta_idx: 0,
        }
    }

    pub fn update(
        &mut self,
        error: f32,
        delta_target: f32,
        config: &PidConfig,
        interval: Duration,
    ) -> PidResult {
        let cfg = config;
        let interval = interval.as_secs_f32();

        self.integral += error * interval;
        self.integral = self.integral.clamp(-cfg.max_integral, cfg.max_integral);

        let proportional = error;
        let integral = self.integral;
        let derivative = (error - self.last_error.unwrap_or(error)) / interval;

        self.last_deltas[self.delta_idx % self.last_deltas.len()] = delta_target;
        let avg_delta_target = self.last_deltas.iter().sum::<f32>() / self.last_deltas.len() as f32;
        self.delta_idx += 1;

        self.last_error = Some(error);

        let p = cfg.kp * proportional;
        let i = cfg.ki * integral;
        let d = cfg.kd * derivative;
        let td = cfg.kt
            * avg_delta_target
                .abs()
                .max(delta_target.abs())
                .copysign(delta_target);

        let correction = p + i + d + td;

        PidResult {
            p,
            i,
            d,
            td,
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
