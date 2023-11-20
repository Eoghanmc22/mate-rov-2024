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
