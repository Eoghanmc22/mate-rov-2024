use anyhow::Context;
use bevy::{
    app::{App, Plugin},
    core,
    prelude::{EntityRef, EntityWorldMut, World},
};
use opencv::{
    core::{Point, Scalar},
    imgproc,
    prelude::*,
};

use crate::video_pipelines::{AppPipelineExt, Pipeline, PipelineCallbacks};

pub struct EdgesPipelinePlugin;

impl Plugin for EdgesPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<EdgesPipeline>();
    }
}

#[derive(Default)]
struct EdgesPipeline {
    edges: Mat,
}

impl Pipeline for EdgesPipeline {
    const NAME: &'static str = "Edge Detection Pipeline";

    type Input = ();

    fn collect_inputs(world: &World, entity: &EntityRef) -> Self::Input {
        // No-op
    }

    fn process<'b, 'a: 'b>(
        &'a mut self,
        cmds: PipelineCallbacks,
        data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b Mat> {
        opencv::imgproc::canny(img, &mut self.edges, 150.0, 150.0, 3, false).context("Canny")?;
        Ok(&self.edges)
    }

    fn cleanup(entity_world: &mut EntityWorldMut) {
        // No-op
    }
}
