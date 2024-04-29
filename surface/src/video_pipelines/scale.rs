use anyhow::Context;
use bevy::{
    app::{App, Plugin},
    prelude::{EntityRef, EntityWorldMut, World},
};
use opencv::{core::Size, imgproc, prelude::*};

use crate::video_pipelines::{AppPipelineExt, Pipeline, PipelineCallbacks};

pub struct ScalePipelinePlugin;

impl Plugin for ScalePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<ScalePipeline<2, -2>>("1/4 Scale Pipeline");
    }
}

#[derive(Default)]
pub struct ScalePipeline<const BASE: u32, const EXPONENT: i32> {
    scaled: Mat,
}

impl<const BASE: u32, const EXPONENT: i32> Pipeline for ScalePipeline<BASE, EXPONENT> {
    type Input = ();

    fn collect_inputs(_world: &World, _entity: &EntityRef) -> Self::Input {
        // No-op
    }

    fn process<'b, 'a: 'b>(
        &'a mut self,
        _cmds: &mut PipelineCallbacks,
        _data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b mut Mat> {
        let scale = f64::powi(BASE as f64, EXPONENT);

        let interpolation = if scale >= 1.0 {
            imgproc::INTER_LINEAR
        } else {
            imgproc::INTER_AREA
        };

        imgproc::resize(
            img,
            &mut self.scaled,
            Size::default(),
            scale,
            scale,
            interpolation,
        )
        .context("Resize")?;

        Ok(&mut self.scaled)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // No-op
    }
}
