pub mod motor_math;
pub mod motor_setup;
pub mod pwm;
// TODO: Depth control
// TODO: Orientation control

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub struct MovementPlugins;

impl PluginGroup for MovementPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(motor_setup::MotorSetupPlugin)
            .add(motor_math::MotorMathPlugin)
            .add(pwm::PwmOutputPlugin)
            .build()
    }
}
