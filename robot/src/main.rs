pub mod peripheral;
pub mod plugins;

use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use plugins::{
    ctrlc::CtrlCPlugin, orientation::OrientationPlugin, robot::RobotPlugin, sync::SyncPlugin,
};
use tracing::Level;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // TODO: Make sure commands from Update get flushed before the network write system runs in PostUpdate

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 100.0,
            ))),
        )
        .add_plugins((SyncPlugin, CtrlCPlugin, RobotPlugin, OrientationPlugin))
        .run();
}
