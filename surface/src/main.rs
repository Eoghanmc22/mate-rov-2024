pub mod attitude;
pub mod input;
pub mod surface;
pub mod ui;
pub mod video_display;
pub mod video_stream;

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
use common::{sync::SyncRole, CommonPlugins};
use input::InputPlugin;
use surface::SurfacePlugin;
use ui::EguiUiPlugin;
use video_display::VideoDisplayPlugin;
use video_stream::VideoStreamPlugin;

fn main() -> anyhow::Result<()> {
    // TODO/FIXME: Times out when focus is lost
    App::new()
        .add_plugins((
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
            LogDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin,
            FrameTimeDiagnosticsPlugin,
            SystemInformationDiagnosticsPlugin,
            TokioTasksPlugin::default(),
            CommonPlugins(SyncRole::Client).build(),
            SurfacePlugin,
            InputPlugin,
            EguiUiPlugin,
            AttitudePlugin,
            VideoStreamPlugin,
            VideoDisplayPlugin,
            // TODO: Way to close and re open
            WorldInspectorPlugin::new(),
            PanOrbitCameraPlugin,
        ))
        .run();

    Ok(())
}
