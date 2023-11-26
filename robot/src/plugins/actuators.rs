pub mod depth_hold;
pub mod motor_math;
pub mod motor_setup;
pub mod pwm;
pub mod stabilize;

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

pub struct MovementPlugins;

impl PluginGroup for MovementPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(motor_setup::MotorSetupPlugin)
            .add(motor_math::MotorMathPlugin)
            .add(pwm::PwmOutputPlugin)
            .add(stabilize::StabilizePlugin)
            .add(depth_hold::DepthHoldPlugin)
            .build()
    }
}
