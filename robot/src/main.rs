#![feature(coroutines, iter_from_coroutine)]
#![allow(private_interfaces, clippy::redundant_pattern_matching)]

pub mod config;
pub mod peripheral;
pub mod plugins;

use std::{fs, time::Duration};

use anyhow::Context;
use bevy::{
    app::ScheduleRunnerPlugin,
    diagnostic::{DiagnosticsPlugin, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    log::LogPlugin,
    prelude::*,
};
use common::{sync::SyncRole, CommonPlugins};
use config::RobotConfig;
use plugins::{actuators::MovementPlugins, core::CorePlugins, monitor::MonitorPlugins};

#[cfg(rpi)]
use crate::plugins::sensors::SensorPlugins;

// TODO: LogPlugin now exposes a way to play with the tracing subscriber
fn main() -> anyhow::Result<()> {
    info!("---------- Starting Robot Code ----------");

    info!("Reading config");
    let config = fs::read_to_string("robot.toml").context("Read config")?;
    let config: RobotConfig = toml::from_str(&config).context("Parse config")?;

    let name = config.name.clone();
    let port = config.port;

    info!("Starting bevy");
    App::new()
        .insert_resource(config)
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 100.0,
            ))),
            // .set(TaskPoolPlugin {
            //     task_pool_options: TaskPoolOptions {
            //         compute: TaskPoolThreadAssignmentPolicy {
            //             // set the minimum # of compute threads
            //             // to the total number of available threads
            //             min_threads: available_parallelism(),
            //             max_threads: std::usize::MAX, // unlimited max threads
            //             percent: 1.0,                 // this value is irrelevant in this case
            //         },
            //         // keep the defaults for everything else
            //         ..default()
            //     },
            // })
            // Logging
            LogPlugin::default(),
            // Diagnostics
            (
                DiagnosticsPlugin,
                EntityCountDiagnosticsPlugin,
                FrameTimeDiagnosticsPlugin,
            ),
            // MATE
            (
                CommonPlugins {
                    role: SyncRole::Server { port },
                    name,
                },
                CorePlugins,
                #[cfg(rpi)]
                SensorPlugins,
                MovementPlugins,
                MonitorPlugins,
            ),
        ))
        .run();

    info!("---------- Robot Code Exited Cleanly ----------");

    Ok(())
}
