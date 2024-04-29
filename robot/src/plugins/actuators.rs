pub mod depth_hold;
pub mod leds;
pub mod pwm;
pub mod servo;
pub mod stabilize;
pub mod thruster;

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub struct MovementPlugins;

impl PluginGroup for MovementPlugins {
    fn build(self) -> PluginGroupBuilder {
        let plugins = PluginGroupBuilder::start::<Self>()
            .add(servo::ServoPlugin)
            .add(thruster::ThrusterPlugin)
            .add(stabilize::StabilizePlugin)
            .add(depth_hold::DepthHoldPlugin);

        #[cfg(rpi)]
        let plugins = plugins
            // Plugins depending on robot hardware
            .add(pwm::PwmOutputPlugin)
            .add(leds::LedPlugin);

        plugins
    }
}
