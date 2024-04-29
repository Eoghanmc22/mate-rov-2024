use anyhow::Context;
use bevy::{
    app::{App, Plugin},
    prelude::{EntityRef, EntityWorldMut, World},
};
use opencv::{imgproc, prelude::*};

use crate::video_pipelines::{AppPipelineExt, Pipeline, PipelineCallbacks};

pub struct EdgesPipelinePlugin;

impl Plugin for EdgesPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<EdgesPipeline>("Edge Detection Pipeline");
    }
}

#[derive(Default)]
pub struct EdgesPipeline {
    edges: Mat,
}

impl Pipeline for EdgesPipeline {
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
        imgproc::canny_def(img, &mut self.edges, 150.0, 150.0).context("Canny")?;

        Ok(&mut self.edges)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // No-op
    }
}
