pub mod input;
pub mod surface;

use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use common::{sync::SyncRole, CommonPlugins};
use input::InputPlugin;
use surface::SurfacePlugin;
use tracing::Level;

fn main() -> anyhow::Result<()> {
    // TODO/FIXME: Times out when focus is lost
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<bevy::audio::AudioPlugin>(),
            CommonPlugins(SyncRole::Client).build(),
            WorldInspectorPlugin::new(),
            SurfacePlugin,
            InputPlugin,
        ))
        .run();

    Ok(())
}
