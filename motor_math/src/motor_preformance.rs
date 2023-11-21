use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use crate::Direction;

pub struct MotorData {
    force_index: Vec<MotorRecord>,
    current_index: Vec<MotorRecord>,
}

impl MotorData {
    pub fn lookup_by_force(&self, force: f32, interpolation: Interpolation) -> MotorRecord {
        let partition_point = self.force_index.partition_point(|x| x.force < force);

        let idx_b = partition_point.max(1).min(self.force_index.len() - 1);
        let idx_a = idx_b - 1;

        let a = &self.force_index[idx_a];
        let b = &self.force_index[idx_b];

        Self::interpolate(a, b, force, a.force, b.force, interpolation)
    }

    pub fn lookup_by_current(
        &self,
        signed_current: f32,
        interpolation: Interpolation,
    ) -> MotorRecord {
        let partition_point = self
            .current_index
            .partition_point(|x| x.current.copysign(x.force) < signed_current);

        let idx_b = partition_point.max(1).min(self.current_index.len() - 1);
        let idx_a = idx_b - 1;

        let a = &self.current_index[idx_a];
        let b = &self.current_index[idx_b];

        Self::interpolate(
            a,
            b,
            signed_current,
            a.current.copysign(a.force),
            b.current.copysign(b.force),
            interpolation,
        )
    }

    fn interpolate(
        a: &MotorRecord,
        b: &MotorRecord,
        value: f32,
        value_a: f32,
        value_b: f32,
        interpolation: Interpolation,
    ) -> MotorRecord {
        let record = match interpolation {
            Interpolation::LerpDirection(_) | Interpolation::Lerp => {
                let alpha = (value - value_a) / (value_b - value_a);
                a.lerp(b, alpha)
            }
            Interpolation::Direction(_) | Interpolation::OriginalData => {
                let dist_a = (value_a - value).abs();
                let dist_b = (value_b - value).abs();

                if dist_a <= dist_b {
                    *a
                } else {
                    *b
                }
            }
        };

        match interpolation {
            Interpolation::LerpDirection(direction) | Interpolation::Direction(direction) => {
                if let Direction::CounterClockwise = direction {
                    MotorRecord {
                        pwm: 3000.0 - record.pwm,
                        ..record
                    }
                } else {
                    record
                }
            }
            Interpolation::Lerp | Interpolation::OriginalData => record,
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

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Interpolation {
    /// Return the linear interpolation betwwn the two data entries closest to the the requested data point
    /// and modifies the pwm field to match the direction of the propeller
    LerpDirection(Direction),
    /// Return the raw data entry closest to the the requested data point
    /// Only modifies the pwm field to match the direction of the propeller
    Direction(Direction),
    /// Return the linear interpolation betwwn the two data entries closest to the the requested data point
    #[default]
    Lerp,
    /// Return the raw data entry closest to the the requested data point
    /// Make no modifications to the data
    OriginalData,
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
    pub fn lerp(&self, other: &Self, alpha: f32) -> Self {
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
