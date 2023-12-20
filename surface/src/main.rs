pub mod attitude;
pub mod input;
pub mod surface;
pub mod ui;

use attitude::AttitudePlugin;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_tokio_tasks::TokioTasksPlugin;
use common::{sync::SyncRole, CommonPlugins};
use input::InputPlugin;
use surface::SurfacePlugin;
use ui::EguiUiPlugin;

fn main() -> anyhow::Result<()> {
    // TODO/FIXME: Times out when focus is lost
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<bevy::audio::AudioPlugin>(),
            TokioTasksPlugin::default(),
            CommonPlugins(SyncRole::Client).build(),
            SurfacePlugin,
            InputPlugin,
            EguiUiPlugin,
            AttitudePlugin,
            WorldInspectorPlugin::new(),
        ))
        .run();

    Ok(())
}
