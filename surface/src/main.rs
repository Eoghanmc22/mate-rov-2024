pub mod attitude;
pub mod input;
pub mod surface;
pub mod ui;
pub mod video_display;
pub mod video_stream;

use std::time::Duration;

use attitude::AttitudePlugin;
use bevy::{
    core::TaskPoolThreadAssignmentPolicy,
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
    tasks::available_parallelism,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_tokio_tasks::TokioTasksPlugin;
use common::{over_run::OverRunSettings, sync::SyncRole, CommonPlugins};
use input::InputPlugin;
use surface::SurfacePlugin;
use ui::EguiUiPlugin;
use video_display::VideoDisplayPlugin;
use video_stream::VideoStreamPlugin;

fn main() -> anyhow::Result<()> {
    // FIXME(high): Times out when focus is lost
    App::new()
        .insert_resource(OverRunSettings {
            max_time: Duration::from_secs_f32(1.0 / 60.0),
        })
        .add_plugins((
            // Bevy Core
            DefaultPlugins
                .build()
                .disable::<bevy::audio::AudioPlugin>()
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        compute: TaskPoolThreadAssignmentPolicy {
                            // set the minimum # of compute threads
                            // to the total number of available threads
                            min_threads: available_parallelism(),
                            max_threads: std::usize::MAX, // unlimited max threads
                            percent: 1.0,                 // this value is irrelevant in this case
                        },
                        // keep the defaults for everything else
                        ..default()
                    },
                }),
            // Diagnostics
            (
                LogDiagnosticsPlugin::default(),
                EntityCountDiagnosticsPlugin,
                FrameTimeDiagnosticsPlugin,
                SystemInformationDiagnosticsPlugin,
            ),
            // MATE
            (
                CommonPlugins(SyncRole::Client).build(),
                SurfacePlugin,
                InputPlugin,
                EguiUiPlugin,
                AttitudePlugin,
                VideoStreamPlugin,
                VideoDisplayPlugin,
            ),
            // 3rd Party
            (
                TokioTasksPlugin::default(),
                // TODO(high): Way to close and re open
                WorldInspectorPlugin::new(),
                PanOrbitCameraPlugin,
            ),
        ))
        .run();

    Ok(())
}
