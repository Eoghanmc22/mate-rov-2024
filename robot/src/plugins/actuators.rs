pub mod depth_hold;
pub mod leds;
pub mod motor_math;
pub mod motor_setup;
pub mod pwm;
pub mod stabilize;

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub struct MovementPlugins;

impl PluginGroup for MovementPlugins {
    fn build(self) -> PluginGroupBuilder {
        let plugins = PluginGroupBuilder::start::<Self>()
            .add(motor_setup::MotorSetupPlugin)
            .add(motor_math::MotorMathPlugin)
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
