use std::{
    fmt::{Display, Formatter},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use bevy::app::App;
use bevy::reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

macro_rules! unit {
    ($name:ident, $repr:ty, $fmt:expr) => {
        #[derive(
            Debug, Copy, Clone, Default, Serialize, Deserialize, Reflect, PartialOrd, PartialEq,
        )]
        #[reflect(Serialize, Deserialize, Debug, PartialEq, Default)]
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

        impl From<$repr> for $name {
            fn from(value: $repr) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $repr {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

macro_rules! units {
    ($($name:ident, $fmt:expr);* ) => {
        $(
            unit!($name, Repr, $fmt);
        )*

        pub fn register_types(app: &mut App) {
            $(
                app.register_type::<$name>();
            )*
        }
    }
}

type Repr = f32;

units! {
    Meters, "{:.2}M";
    Mbar, "{:.2}mbar";
    Celsius, "{:.2}°C";
    GForce, "{:.2}g";
    Radians, "{:.2}rad";
    Degrees, "{:.2}°";
    Dps, "{:.2}°/s";
    Gauss, "{:.2}Gs";
    Newtons, "{:.2}N";
    Volts, "{:.2}V";
    Amperes, "{:.2}A"
}
