#![allow(private_interfaces, clippy::redundant_pattern_matching)]

pub mod config;
pub mod peripheral;
pub mod plugins;

use std::{fs, time::Duration};

use anyhow::Context;
use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use config::RobotConfig;
use plugins::{
    actuators::MovementPlugins, core::CorePlugins, monitor::MonitorPlugins, sensors::SensorPlugins,
};
use tracing::Level;

fn main() -> anyhow::Result<()> {
    // TODO: Rotating log file
    // TODO: tracy support
    // TODO: Could tracing replace the current error system?
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let config = fs::read_to_string("robot.toml").context("Read config")?;
    let config: RobotConfig = toml::from_str(&config).context("Parse config")?;

    // TODO: Make sure commands from Update get flushed before the network write system runs in PostUpdate

    let bevy_plugins = MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 100.0,
    )));

    App::new()
        .insert_resource(config)
        .add_plugins((
            bevy_plugins,
            CorePlugins,
            SensorPlugins,
            MovementPlugins,
            MonitorPlugins,
        ))
        .run();

    Ok(())
}
