use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

pub struct MotorData {
    pub force_index: Vec<MotorRecord>,
    pub current_index: Vec<MotorRecord>,
}

impl MotorData {
    // TODO: Fix handeling for extreme values
    pub fn lookup_by_force(&self, force: f32, interpolate: bool) -> MotorRecord {
        // FIXME: Subtraction can under flow
        let idx = self.force_index.partition_point(|x| x.force < force) - 1;
        // FIXME: This can panic
        let val = self.force_index[idx];

        if interpolate && idx + 1 < self.force_index.len() {
            let next = &self.force_index[idx + 1];
            let alpha = (force - val.force) / (next.force - val.force);

            val.interpolate(next, alpha)
        } else {
            val
        }
    }

    // TODO: Fix handeling for extreme values
    pub fn lookup_by_current(&self, signed_current: f32, interpolate: bool) -> MotorRecord {
        // FIXME: Subtraction can under flow
        let idx = self
            .current_index
            .partition_point(|x| x.current.copysign(x.force) < signed_current)
            - 1;
        // FIXME: This can panic
        let val = self.current_index[idx];

        if interpolate && idx + 1 < self.force_index.len() {
            let next = &self.current_index[idx + 1];
            let alpha = (signed_current - val.current.copysign(val.force))
                / (next.current.copysign(next.force) - val.current.copysign(val.force));

            val.interpolate(next, alpha)
        } else {
            val
        }
    }
}

impl From<Vec<MotorRecord>> for MotorData {
    fn from(value: Vec<MotorRecord>) -> Self {
        let mut force_index = value.clone();

        force_index.sort_by(|a, b| f32::total_cmp(&a.force, &b.force));
        force_index.dedup_by_key(|it| it.force);

        let mut current_index = value.clone();

        current_index.sort_by(|a, b| {
            f32::total_cmp(&a.current.copysign(a.force), &b.current.copysign(b.force))
        });
        current_index.dedup_by_key(|it| it.current.copysign(it.force));

        Self {
            force_index,
            current_index,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct MotorRecord {
    pub pwm: f32,
    pub rpm: f32,
    pub current: f32,
    pub voltage: f32,
    pub power: f32,
    pub force: f32,
    pub efficiency: f32,
}

impl MotorRecord {
    pub fn interpolate(&self, other: &Self, alpha: f32) -> Self {
        debug_assert!((0.0..=1.0).contains(&alpha));

        Self {
            pwm: lerp(self.pwm, other.pwm, alpha),
            rpm: lerp(self.rpm, other.rpm, alpha),
            current: lerp(self.current, other.current, alpha),
            voltage: lerp(self.voltage, other.voltage, alpha),
            power: lerp(self.power, other.power, alpha),
            force: lerp(self.force, other.force, alpha),
            efficiency: lerp(self.efficiency, other.efficiency, alpha),
        }
    }
}

fn lerp(a: f32, b: f32, alpha: f32) -> f32 {
    (1.0 - alpha) * a + alpha * b
}

pub fn read_motor_data<P: AsRef<Path>>(path: P) -> anyhow::Result<MotorData> {
    let csv = csv::Reader::from_path(path).context("Read data")?;

    let mut data = Vec::default();
    for result in csv.into_deserialize() {
        let record: MotorRecord = result.context("Parse motor record")?;
        data.push(record);
    }

    Ok(data.into())
}
