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

// TODO Fix motor math
pub fn mix_movement<'a>(mov: Movement, motor_data: &MotorData) -> HashMap<MotorId, MotorFrame> {
    const MAX_AMPERAGE: f64 = 20.0;

    let drive_ids = [
        MotorId::FrontLeftBottom,
        MotorId::FrontLeftTop,
        MotorId::FrontRightBottom,
        MotorId::FrontRightTop,
        MotorId::BackLeftBottom,
        MotorId::BackLeftTop,
        MotorId::BackRightBottom,
        MotorId::BackRightTop,
    ];
    let servo_ids = [
        MotorId::Camera1,
        MotorId::Camera2,
        MotorId::Camera3,
        MotorId::Camera4,
        MotorId::Aux1,
        MotorId::Aux2,
        MotorId::Aux3,
        MotorId::Aux4,
    ];

    let Movement {
        x,
        y,
        z,
        x_rot,
        y_rot,
        z_rot,
        cam_1,
        cam_2,
        cam_3,
        cam_4,
        aux_1,
        aux_2,
        aux_3,
        aux_4,
    } = mov;

    let (x, y, z) = (x.get(), y.get(), z.get());
    let (x_rot, y_rot, z_rot) = (x_rot.get(), y_rot.get(), z_rot.get());

    let mut raw_mix = HashMap::default();

    for motor_id in drive_ids {
        let motor = Motor::from(motor_id);

        #[rustfmt::skip]
        let speed = match motor_id {
            MotorId::FrontLeftBottom =>   -x - y + z + x_rot + y_rot - z_rot,
            MotorId::FrontLeftTop =>      -x - y - z - x_rot - y_rot - z_rot,
            MotorId::FrontRightBottom =>   x - y + z + x_rot - y_rot + z_rot,
            MotorId::FrontRightTop =>      x - y - z - x_rot + y_rot + z_rot,
            MotorId::BackLeftBottom =>    -x + y + z - x_rot + y_rot + z_rot,
            MotorId::BackLeftTop =>       -x + y - z + x_rot - y_rot + z_rot,
            MotorId::BackRightBottom =>    x + y + z - x_rot - y_rot - z_rot,
            MotorId::BackRightTop =>       x + y - z + x_rot + y_rot - z_rot,

            _ => unreachable!()
        };

        let skew = if speed >= 0.0 { 1.0 } else { 1.25 };
        let direction = motor.max_value.get().signum();

        raw_mix.insert(motor_id, speed * skew * direction);
    }

    let max_raw = raw_mix.len() as f64;
    let total_raw: f64 = raw_mix.values().map(|it| it.abs()).sum();
    let scale_raw = if total_raw > max_raw {
        max_raw / total_raw
    } else {
        // Handle cases where we dont want to go max speed
        1.0
    };

    let motor_amperage = MAX_AMPERAGE / max_raw;
    let mut speeds: HashMap<MotorId, MotorFrame> = raw_mix
        .into_iter()
        .map(|(motor, value)| (motor, value * scale_raw * motor_amperage))
        .map(|(motor, current)| (motor, motor_data.pwm_for_current(current)))
        .map(|(motor, pwm)| (motor, MotorFrame::Raw(pwm)))
        .collect();

    for motor in servo_ids {
        #[rustfmt::skip]
        let speed = match motor {
            MotorId::Camera1 => cam_1,
            MotorId::Camera2 => cam_2,
            MotorId::Camera3 => cam_3,
            MotorId::Camera4 => cam_4,

            MotorId::Aux1 => aux_1,
            MotorId::Aux2 => aux_2,
            MotorId::Aux3 => aux_3,
            MotorId::Aux4 => aux_4,

            _ => unreachable!()
        };

        speeds.insert(motor, MotorFrame::Percent(speed));
    }

    speeds
}

pub struct MotorData {
    forward: Vec<MotorRecord>,
    backward: Vec<MotorRecord>,
}

impl MotorData {
    pub fn sort(&mut self) {
        self.forward
            .sort_by(|a, b| f64::total_cmp(&a.current, &b.current));
        self.backward
            .sort_by(|a, b| f64::total_cmp(&a.current, &b.current));
    }

    pub fn pwm_for_current(&self, signed_current: f64) -> Duration {
        let current = signed_current.abs();

        let data_set = if signed_current >= 0.0 {
            &self.forward
        } else {
            &self.backward
        };
        assert!(!data_set.is_empty());

        let idx = data_set.partition_point(|x| x.current < current);
        let pwm = if idx > 0 && idx < data_set.len() {
            let a = &data_set[idx - 1];
            let b = &data_set[idx];

            let alpha = (current - a.current) / (b.current - a.current);

            a.pwm * (1.0 - alpha) + (b.pwm * alpha)
        } else {
            data_set[0].pwm
        };

        Duration::from_micros(pwm as u64)
    }
}

#[derive(Deserialize, Debug)]
pub struct MotorRecord {
    pwm: f64,
    rpm: f64,
    current: f64,
    voltage: f64,
    power: f64,
    force: f64,
    efficiency: f64,
}

pub fn read_motor_data() -> anyhow::Result<MotorData> {
    let forward = csv::Reader::from_path("forward_motor_data.csv").context("Read forward data")?;
    let reverse = csv::Reader::from_path("reverse_motor_data.csv").context("Read reverse data")?;

    let mut forward_data = Vec::default();
    for result in forward.into_deserialize() {
        let record: MotorRecord = result.context("Parse motor record")?;
        forward_data.push(record);
    }

    let mut reverse_data = Vec::default();
    for result in reverse.into_deserialize() {
        let record: MotorRecord = result.context("Parse motor record")?;
        reverse_data.push(record);
    }

    let mut motor_data = MotorData {
        forward: forward_data,
        backward: reverse_data,
    };
    motor_data.sort();

    Ok(motor_data)
}
