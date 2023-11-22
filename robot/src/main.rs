#![allow(private_interfaces, clippy::redundant_pattern_matching)]

pub mod peripheral;
pub mod plugins;

use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use plugins::{
    actuators::MovementPlugins, core::CorePlugins, monitor::MonitorPlugins, sensors::SensorPlugins,
};
use tracing::Level;

fn main() {
    // TODO: Rotating log file
    // TODO: tracy support
    // TODO: Could tracing replace the current error system?
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // TODO: Make sure commands from Update get flushed before the network write system runs in PostUpdate

    let bevy_plugins = MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 100.0,
    )));

    App::new()
        .add_plugins((
            bevy_plugins,
            CorePlugins,
            SensorPlugins,
            MovementPlugins,
            MonitorPlugins,
        ))
        .run();
}
