//! Code shared between both the surface and robot projects
#![feature(try_blocks, const_fn_floating_point_arithmetic, const_float_classify)]
#![allow(clippy::type_complexity)]

pub mod adapters;
pub mod components;
pub mod ecs_sync;
pub mod protocol;
pub mod token;
pub mod types;
