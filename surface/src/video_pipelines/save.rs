use anyhow::Context;
use bevy::{
    app::{App, Plugin},
    prelude::{EntityRef, EntityWorldMut, World},
};
use opencv::{imgcodecs, prelude::*};
use time::format_description::well_known::Iso8601;

use crate::video_pipelines::{AppPipelineExt, Pipeline, PipelineCallbacks};

pub struct SavePipelinePlugin;

impl Plugin for SavePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<SavePipeline>("Save Pipeline");
    }
}

#[derive(Default)]
pub struct SavePipeline;

impl Pipeline for SavePipeline {
    type Input = ();

    fn collect_inputs(_world: &World, _entity: &EntityRef) -> Self::Input {
        // No-op
    }

    fn process<'b, 'a: 'b>(
        &'a mut self,
        cmds: &mut PipelineCallbacks,
        _data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b mut Mat> {
        cmds.should_end();
        let time = time::OffsetDateTime::now_utc();
        let file_name = time.format(&Iso8601::DATE_TIME).context("Format time")?;
        imgcodecs::imwrite_def(&format!("img_{file_name}.png"), img).context("Write screenshot")?;

        Ok(img)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // No-op
    }
}
