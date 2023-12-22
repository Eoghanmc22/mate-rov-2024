pub mod attitude;
pub mod input;
pub mod surface;
pub mod ui;
pub mod video_display;
pub mod video_stream;
pub mod xr;

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
use bevy_oxr::DefaultXrPlugins;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_tokio_tasks::TokioTasksPlugin;
use common::{sync::SyncRole, CommonPlugins};
use input::InputPlugin;
use surface::SurfacePlugin;
use ui::EguiUiPlugin;
use video_display::VideoDisplayPlugin;
use video_stream::VideoStreamPlugin;
use xr::OpenXrPlugin;

// TODO/FIXME: Times out when focus is lost
fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugins((
            // Bevy Core
            // DefaultPlugins
            DefaultXrPlugins
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
                OpenXrPlugin,
            ),
            // 3rd Party
            (
                TokioTasksPlugin::default(),
                // TODO: Way to close and re open
                WorldInspectorPlugin::new(),
                PanOrbitCameraPlugin,
            ),
        ))
        .run();

    Ok(())
}
