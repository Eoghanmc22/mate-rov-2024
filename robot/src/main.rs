// pub mod peripheral;
// pub mod systems;

pub mod plugins;

use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use plugins::{ctrlc::CtrlCPlugin, sync::SyncPlugin};
use tracing::Level;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 100.0,
            ))),
        )
        .add_plugins((SyncPlugin, CtrlCPlugin))
        .run();
}
