use ahash::HashMap;
use common::types::units::Percent;
use std::fmt::Debug;
use std::time::Duration;

const DEFAULT_MOTOR_CW: Motor = Motor {
    channel: 255,
    max_value: Percent::new(0.45), // Full speed on all motors would blow fuse
    // Taken from basic esc spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};
const DEFAULT_MOTOR_CCW: Motor = Motor {
    channel: 255,
    max_value: Percent::new(-0.45), // Full speed on all motors would blow fuse
    // Taken from basic esc spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};

const DEFAULT_SERVO: Motor = Motor {
    channel: 255,
    max_value: Percent::new(1.0),
    // Taken from servo spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Motor {
    /// PWM signal channel
    pub channel: u8,

    /// Speed settings, can be negative to reverse direction
    pub max_value: Percent,

    /// PWM info
    pub reverse: Duration,
    pub forward: Duration,
    pub center: Duration,
}

impl Motor {
    #[must_use]
    pub fn value_to_pwm(&self, speed: Percent) -> Duration {
        let speed = speed.get() * self.max_value.get();

        let upper = if speed >= 0.0 {
            self.forward.as_micros()
        } else {
            self.reverse.as_micros()
        };
        let lower = self.center.as_micros();

        let scaled_speed = speed.abs() * 1000.0;
        let pulse = (upper as i64 * scaled_speed as i64
            + lower as i64 * (1000 - scaled_speed as i64))
            / 1000;

        Duration::from_micros(pulse as u64)
    }
}
