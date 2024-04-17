use anyhow::Context;
use bevy::{
    app::{App, Plugin},
    prelude::{Entity, EntityRef, EntityWorldMut, World},
};
use opencv::{
    calib3d,
    core::{self, Range, Rect, Scalar, Size},
    imgproc,
    prelude::*,
};

use crate::video_pipelines::{AppPipelineExt, FromWorldEntity, Pipeline, PipelineCallbacks};

pub struct UndistortPipelinePlugin;

impl Plugin for UndistortPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<UndistortPipeline>("Undistort Pipeline");
    }
}

pub struct UndistortPipeline {
    undistorted: Mat,
    cropped: Mat,

    mtx: Mat,
    dist: Mat,

    remap: Option<RemapData>,
}

struct RemapData {
    size: Size,

    map_x: Mat,
    map_y: Mat,

    rows: Range,
    cols: Range,
}

impl Pipeline for UndistortPipeline {
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
        let size = img.size().context("Get image size")?;

        if let Some(ref mut remap) = self.remap {
            if remap.size != size {
                self.remap = None;
            }
        }

        let UndistortPipeline {
            undistorted,
            cropped,
            mtx,
            dist,
            remap,
        } = self;

        let RemapData {
            map_x,
            map_y,
            rows,
            cols,
            ..
        } = match remap {
            Some(remap) => remap,
            None => {
                let mut roi = Rect::default();
                let new_mtx = calib3d::get_optimal_new_camera_matrix(
                    mtx,
                    dist,
                    size,
                    0.0,
                    size,
                    Some(&mut roi),
                    false,
                )
                .context("Get optimal matrix")?;

                let mut map_x = Mat::default();
                let mut map_y = Mat::default();
                // TODO: What does the 5 mean? taken from https://docs.opencv.org/4.x/dc/dbb/tutorial_py_calibration.html
                calib3d::init_undistort_rectify_map(
                    mtx,
                    dist,
                    &Mat::default(),
                    &new_mtx,
                    size,
                    5,
                    &mut map_x,
                    &mut map_y,
                )
                .context("Init rectify map")?;

                remap.insert(RemapData {
                    size,
                    map_x,
                    map_y,
                    rows: Range::new(roi.x, roi.x + roi.width).context("Rows Range")?,
                    cols: Range::new(roi.y, roi.y + roi.height).context("Cols Range")?,
                })
            }
        };

        opencv::imgproc::remap(
            img,
            undistorted,
            map_x,
            map_y,
            imgproc::INTER_LINEAR,
            core::BORDER_CONSTANT,
            Scalar::default(),
        )
        .context("Remap")?;

        *cropped = undistorted.row_range(rows).context("Crop Rows")?;
        *cropped = cropped.col_range(cols).context("Crop Cols")?;

        Ok(cropped)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // No-op
    }
}

impl FromWorldEntity for UndistortPipeline {
    fn from(world: &mut World, camera: Entity) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        // TODO: Store these values on the robot and grab them from the ecs here
        let mtx = Mat::default();
        let dist = Mat::default();

        // Self {
        //     undistorted: Mat::default(),
        //     cropped: Mat::default(),
        //     mtx,
        //     dist,
        //     remap: None,
        // };
        todo!("Get real data")
    }
}
