#![feature(iter_intersperse)]

pub mod attitude;
pub mod input;
pub mod surface;
pub mod ui;
pub mod video_display_2d_master;
pub mod video_display_2d_tile;
pub mod video_display_3d;
pub mod video_pipelines;
pub mod video_stream;

use std::time::Duration;

use anyhow::Context;
use attitude::AttitudePlugin;
use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_picking::{highlight::DefaultHighlightingPlugin, DefaultPickingPlugins};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_tokio_tasks::TokioTasksPlugin;
use common::{over_run::OverRunSettings, sync::SyncRole, CommonPlugins};
use crossbeam::channel::unbounded;
use input::InputPlugin;
use opencv::{highgui, imgcodecs};
use surface::SurfacePlugin;
use ui::{EguiUiPlugin, ShowInspector};
// use video_display_2d_tile::{VideoDisplay2DPlugin, VideoDisplay2DSettings};
use video_display_2d_master::{VideoDisplay2DPlugin, VideoDisplay2DSettings};
// use video_display_3d::{VideoDisplay3DPlugin, VideoDisplay3DSettings};
use video_stream::VideoStreamPlugin;

use crate::video_pipelines::{
    edges::EdgesPipeline,
    marker::MarkerPipeline,
    measure::{MeasurePipeline, MeasurementTarget},
    Pipeline, PipelineCallbacks, SerialPipeline, VideoPipelinePlugins,
};

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
        // .insert_resource(VideoDisplay3DSettings { enabled: true })
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
                VideoDisplay2DPlugin,
                // VideoDisplay3DPlugin,
                VideoPipelinePlugins,
            ),
            // 3rd Party
            (
                DefaultPickingPlugins
                    .build()
                    .disable::<DefaultHighlightingPlugin>(),
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

fn opencv() -> anyhow::Result<()> {
    let mut img = imgcodecs::imread_def("test.jpg").context("Read image")?;

    let (cmds_tx, cmds_rx) = unbounded();
    let mut should_end = false;
    let mut cmds = PipelineCallbacks {
        cmds_tx: &cmds_tx,
        pipeline_entity: Entity::PLACEHOLDER,
        camera_entity: Entity::PLACEHOLDER,
        should_end: &mut should_end,
    };

    // let mut pipeline: FullMeasurePipeline = SerialPipeline(Default::default());
    let mut pipeline: MeasurePipeline = Default::default();
    let out = pipeline
        .process(
            &mut cmds,
            &Some(MeasurementTarget {
                poi: Vec2::new(643.0 / 1920.0, 913.0 / 1080.0),
                left: Vec2::default(),
                right: Vec2::default(),
            }),
            &mut img,
        )
        .context("Process")?;

    highgui::imshow("Image", out).context("Gui")?;
    highgui::wait_key_def().context("Wait key")?;

    Ok(())
}
