use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use common::{sync::SyncRole, CommonPlugins};
use tracing::Level;

fn main() -> anyhow::Result<()> {
    // TODO: tracy support
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // TODO/FIXME: Times out when focus is lost
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<bevy::audio::AudioPlugin>(),
            CommonPlugins(SyncRole::Client).build(),
            WorldInspectorPlugin::new(),
        ))
        .run();

    Ok(())
}
