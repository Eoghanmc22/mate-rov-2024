#![feature(coroutines, iter_from_coroutine)]
#![allow(private_interfaces, clippy::redundant_pattern_matching)]

pub mod config;
pub mod peripheral;
pub mod plugins;

use std::{fs, time::Duration};

use anyhow::Context;
use bevy::{
    app::ScheduleRunnerPlugin,
    core::TaskPoolThreadAssignmentPolicy,
    diagnostic::{
        DiagnosticsPlugin, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    log::LogPlugin,
    prelude::*,
    tasks::available_parallelism,
};
use common::{sync::SyncRole, CommonPlugins};
use config::RobotConfig;
use plugins::{
    actuators::MovementPlugins, core::CorePlugins, monitor::MonitorPlugins, sensors::SensorPlugins,
};

fn main() -> anyhow::Result<()> {
    let config = fs::read_to_string("robot.toml").context("Read config")?;
    let config: RobotConfig = toml::from_str(&config).context("Parse config")?;

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
            LogPlugin::default(),
            (
                DiagnosticsPlugin,
                EntityCountDiagnosticsPlugin,
                FrameTimeDiagnosticsPlugin,
            ),
            (
                CommonPlugins(SyncRole::Server),
                CorePlugins,
                SensorPlugins,
                MovementPlugins,
                MonitorPlugins,
            ),
        ))
        .run();

    Ok(())
}
