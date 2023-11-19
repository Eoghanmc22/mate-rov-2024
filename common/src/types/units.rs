use std::{
    fmt::{Display, Formatter},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use serde::{Deserialize, Serialize};

macro_rules! unit {
    ($name:ident, $repr:ty, $fmt:expr) => {
        #[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
        pub struct $name(pub $repr);

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.pad(&format!($fmt, self.0))
            }
        }

        impl Add for $name {
            type Output = $name;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }

        impl Sub for $name {
            type Output = $name;

            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 - rhs.0)
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }

        impl Mul<$name> for $name {
            type Output = $name;

            fn mul(self, rhs: $name) -> Self::Output {
                Self(self.0 * rhs.0)
            }
        }

        impl MulAssign<$name> for $name {
            fn mul_assign(&mut self, rhs: $name) {
                *self = *self * rhs;
            }
        }

        impl Div<$name> for $name {
            type Output = $name;

            fn div(self, rhs: $name) -> Self::Output {
                Self(self.0 / rhs.0)
            }
        }

        impl DivAssign<$name> for $name {
            fn div_assign(&mut self, rhs: $name) {
                *self = *self / rhs;
            }
        }

        impl Neg for $name {
            type Output = $name;

            fn neg(self) -> Self::Output {
                $name(-self.0)
            }
        }
    };
}

type Repr = f32;

unit!(Meters, Repr, "{:.2}M");
unit!(Mbar, Repr, "{:.2}mbar");
unit!(Celsius, Repr, "{:.2}°C");
unit!(GForce, Repr, "{:.2}g");
unit!(Radians, Repr, "{:.2}rad");
unit!(Degrees, Repr, "{:.2}°");
unit!(Dps, Repr, "{:.2}°/s");
unit!(Gauss, Repr, "{:.2}Gs");
unit!(Newtons, Repr, "{:.2}N");
unit!(Volts, Repr, "{:.2}V");
unit!(Amperes, Repr, "{:.2}A");

// Unfortuationally percent is a little different :(
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Percent(Repr);

impl Percent {
    pub const MAX_VAL: Percent = Percent(1.0);
    pub const MIN_VAL: Percent = Percent(-1.0);
    pub const ZERO: Percent = Percent(0.0);

    /// Creates a new `Speed`. Input should be between -1.0 and 1.0
    pub const fn new(speed: Repr) -> Self {
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
    pub const fn get(self) -> Repr {
        self.0
    }
}

impl Add<Percent> for Percent {
    type Output = Percent;

    fn add(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 + rhs.0)
    }
}

impl AddAssign for Percent {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub<Percent> for Percent {
    type Output = Percent;

    fn sub(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 - rhs.0)
    }
}

impl SubAssign for Percent {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<Percent> for Percent {
    type Output = Percent;

    fn mul(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 * rhs.0)
    }
}

impl MulAssign<Percent> for Percent {
    fn mul_assign(&mut self, rhs: Percent) {
        *self = *self * rhs;
    }
}

impl Div<Percent> for Percent {
    type Output = Percent;

    fn div(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 / rhs.0)
    }
}

impl DivAssign<Percent> for Percent {
    fn div_assign(&mut self, rhs: Percent) {
        *self = *self / rhs;
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
