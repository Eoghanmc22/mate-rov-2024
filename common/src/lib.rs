//! Code shared between both the surface and robot projects
#![feature(const_fn_floating_point_arithmetic, const_float_classify)]

pub mod adapters;
pub mod components;
pub mod protocol;
pub mod token;
pub mod change_detection;
pub mod ecs_sync;
