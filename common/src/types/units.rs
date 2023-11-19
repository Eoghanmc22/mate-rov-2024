use std::{
    fmt::{Display, Formatter},
    ops::{Add, Neg, Sub},
};

use serde::{Deserialize, Serialize};

// TODO: Make a macro for these

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Meters(pub f64);

impl Display for Meters {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}M", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Mbar(pub f64);

impl Display for Mbar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}mbar", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Celsius(pub f64);

impl Display for Celsius {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}°C", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct GForce(pub f64);

impl Display for GForce {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}g", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Radians(pub f64);

impl Display for Radians {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}rad", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Degrees(pub f64);

impl Display for Degrees {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}deg", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Dps(pub f64);

impl Display for Dps {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}dps", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Gauss(pub f64);

impl Display for Gauss {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}Gs", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Newtons(pub f64);

impl Display for Newtons {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}N", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Volts(pub f64);

impl Display for Volts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}V", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Amperes(pub f64);

impl Display for Amperes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}A", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Percent(f64);

impl Percent {
    pub const MAX_VAL: Percent = Percent(1.0);
    pub const MIN_VAL: Percent = Percent(-1.0);
    pub const ZERO: Percent = Percent(0.0);

    /// Creates a new `Speed`. Input should be between -1.0 and 1.0
    pub const fn new(speed: f64) -> Self {
        if !speed.is_normal() {
            return Self::ZERO;
        }
        Self(speed).clamp(Self::MIN_VAL, Self::MAX_VAL)
    }

    /// Clamps a speed to be between `min` and `max`
    pub const fn clamp(self, min: Percent, max: Percent) -> Percent {
        if self.0 > max.0 {
            max
        } else if self.0 < min.0 {
            min
        } else {
            self
        }
    }

    /// Get the speed as a float between -1.0 and 1.0
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Add<Percent> for Percent {
    type Output = Percent;

    fn add(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 + rhs.0)
    }
}

impl Sub<Percent> for Percent {
    type Output = Percent;

    fn sub(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 - rhs.0)
    }
}

impl Neg for Percent {
    type Output = Percent;

    fn neg(self) -> Self::Output {
        Percent(-self.0)
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}%", self.0 * 100.0))
    }
}
