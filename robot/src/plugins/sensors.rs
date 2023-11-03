use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub mod cameras;
pub mod depth;
pub mod leak;
pub mod orientation;

pub struct SensorPlugins;

impl PluginGroup for SensorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(cameras::CameraPlugin)
            .add(orientation::OrientationPlugin)
            .add(depth::DepthPlugin)
            .add(leak::LeakPlugin)
            .build()
    }
}
