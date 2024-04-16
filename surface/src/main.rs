#![feature(iter_intersperse)]
pub mod attitude;
pub mod input;
pub mod surface;
pub mod ui;
pub mod video_display_2d;
pub mod video_display_3d;
pub mod video_pipelines;
pub mod video_stream;

use std::time::Duration;

use attitude::AttitudePlugin;
use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_tokio_tasks::TokioTasksPlugin;
use common::{over_run::OverRunSettings, sync::SyncRole, CommonPlugins};
use input::InputPlugin;
use surface::SurfacePlugin;
use ui::{EguiUiPlugin, ShowInspector};
use video_display_2d::{VideoDisplay2DPlugin, VideoDisplay2DSettings};
use video_display_3d::{VideoDisplay3DPlugin, VideoDisplay3DSettings};
use video_stream::VideoStreamPlugin;

use crate::video_pipelines::VideoPipelinePlugins;

pub const DARK_MODE: bool = false;

fn main() -> anyhow::Result<()> {
    info!("---------- Starting Control Station ----------");

    // FIXME(high): Times out when focus is lost
    App::new()
        .insert_resource(OverRunSettings {
            max_time: Duration::from_secs_f32(1.0 / 60.0),
            tracy_frame_mark: false,
        })
        .insert_resource(VideoDisplay2DSettings { enabled: true })
        .insert_resource(if DARK_MODE {
            ClearColor(Color::rgb_u8(33, 34, 37))
        } else {
            ClearColor(Color::rgb_u8(240, 238, 233))
        })
        .add_plugins((
            // Bevy Core
            DefaultPlugins.build().disable::<bevy::audio::AudioPlugin>(),
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
            // }),
            // Diagnostics
            (
                LogDiagnosticsPlugin::default(),
                EntityCountDiagnosticsPlugin,
                FrameTimeDiagnosticsPlugin,
            ),
            // MATE
            (
                CommonPlugins {
                    name: "Control Station".to_owned(),
                    role: SyncRole::Client,
                },
                SurfacePlugin,
                InputPlugin,
                EguiUiPlugin,
                AttitudePlugin,
                VideoStreamPlugin,
                // VideoDisplay3DPlugin,
                VideoDisplay2DPlugin,
                VideoPipelinePlugins,
            ),
            // 3rd Party
            (
                TokioTasksPlugin::default(),
                // TODO(high): Way to close and re open
                WorldInspectorPlugin::default().run_if(resource_exists::<ShowInspector>),
                PanOrbitCameraPlugin,
            ),
        ))
        .run();

    info!("---------- Control Station Exited Cleanly ----------");

    Ok(())
}
