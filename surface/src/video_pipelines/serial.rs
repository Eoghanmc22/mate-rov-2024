use bevy::app::{App, Plugin};

use crate::video_pipelines::{
    edges::EdgesPipeline, marker::MarkerPipeline, AppPipelineExt, SerialPipeline,
};

pub struct SerialPipelinePlugin;

impl Plugin for SerialPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<SerialPipeline<(MarkerPipeline, EdgesPipeline)>>(
            "Marker -> Edge Pipeline",
        );
    }
}
