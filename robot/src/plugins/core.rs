use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub mod ctrlc;
pub mod error;
pub mod robot;
pub mod state;
pub mod sync;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(robot::RobotPlugin)
            .add(state::StatePlugin)
            .add(error::ErrorPlugin)
            .add(sync::SyncPlugin)
            .add(ctrlc::CtrlCPlugin)
            .build()
    }
}
