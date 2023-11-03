use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub struct MovementPlugins;

impl PluginGroup for MovementPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // TODO
            .build()
    }
}
