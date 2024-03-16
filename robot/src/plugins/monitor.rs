use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub mod hw_stat;

pub struct MonitorPlugins;

impl PluginGroup for MonitorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(hw_stat::HwStatPlugin)
    }
}
