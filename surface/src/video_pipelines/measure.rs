use std::mem;

use anyhow::Context;
use bevy::{
    app::{App, Plugin},
    ecs::component::Component,
    math::Vec2,
    prelude::{EntityRef, EntityWorldMut, World},
};
use opencv::{
    core::{
        Point, Point2f, Rect, Rect2f, RotatedRect, Scalar, Size, Size2f, Vec2f, Vec4f, VecN, Vector,
    },
    imgproc::{self, moments},
    prelude::*,
    types::{VectorOfVectorOfPoint, VectorOfVectorOfPoint2f},
};

use crate::video_pipelines::{
    edges::EdgesPipeline, scale::ScalePipeline, undistort::UndistortPipeline, AppPipelineExt,
    Pipeline, PipelineCallbacks, SerialPipeline,
};

pub struct MeasurePipelinePlugin;

impl Plugin for MeasurePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<MeasurePipeline>("Measure Pipeline");
    }
}

const CONTOUR_MIN_AREA: f64 = 20.0;
const ROI_FACTOR: f32 = 0.75;

/// Percentage
#[derive(Component, Clone, Copy)]
pub struct MeasurementTarget {
    pub poi: Vec2,
    pub left: Vec2,
    pub right: Vec2,
}

#[derive(Clone, Copy)]
struct MeasurementTargetOpenCv {
    poi: Point2f,
    left: Point2f,
    right: Point2f,
}

impl From<MeasurementTarget> for MeasurementTargetOpenCv {
    fn from(value: MeasurementTarget) -> Self {
        MeasurementTargetOpenCv {
            poi: Point2f::new(value.poi.x, value.poi.y),
            left: Point2f::new(value.left.x, value.left.y),
            right: Point2f::new(value.right.x, value.right.y),
        }
    }
}

#[derive(Default)]
pub struct MeasurePipeline {
    blur: Mat,
    edges: Mat,
    contours: VectorOfVectorOfPoint,

    output: Mat,
}

impl Pipeline for MeasurePipeline {
    type Input = Option<MeasurementTarget>;

    fn collect_inputs(_world: &World, entity: &EntityRef) -> Self::Input {
        entity.get::<MeasurementTarget>().copied()
    }

    // TODO: Make the api useful for breaking this up
    fn process<'b, 'a: 'b>(
        &'a mut self,
        _cmds: &mut PipelineCallbacks,
        data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b mut Mat> {
        self.contours.clear();

        let Some(data) = data else {
            return Ok(img);
        };
        let MeasurementTargetOpenCv { poi, left, right } = (*data).into();

        let img_size = img.size().context("Image size")?;

        let (poi, left, right) = {
            let Size2f { width, height } = img_size.to::<f32>().context("Convert size")?;

            let poi = Point2f::new(poi.x * width, poi.y * height);
            let left = Point2f::new(left.x * width, left.y * height);
            let right = Point2f::new(right.x * width, right.y * height);

            (poi, left, right)
        };

        // imgproc::blur_def(img, &mut self.blur, Size::new(3, 3)).context("Blur")?;
        imgproc::canny_def(img, &mut self.edges, 100.0, 100.0).context("Canny")?;
        imgproc::find_contours_def(
            &self.edges,
            &mut self.contours,
            imgproc::RETR_LIST,
            // TODO: Are the other approximation modes better
            imgproc::CHAIN_APPROX_SIMPLE,
        )
        .context("Find contours")?;

        println!("Found {} contours", self.contours.len());

        let mut good_contours = VectorOfVectorOfPoint::new();
        let mut best_contour = None;

        for (idx, contour) in self.contours.iter().enumerate() {
            let moments = imgproc::moments_def(&contour).context("Get moments")?;
            let area = moments.m00;

            // Contour too small
            if area < CONTOUR_MIN_AREA {
                continue;
            }

            // TODO: Might be hard to get a point in the region
            let rst = imgproc::point_polygon_test(&contour, poi, false).context("Point test")?;

            // POI is not in contour
            if rst == -1.0 {
                // continue;
            }

            let c_x = moments.m10 / moments.m00;
            let c_y = moments.m01 / moments.m00;

            let distance = (c_x as f32 - poi.x).powi(2) + (c_y as f32 - poi.y).powi(2);

            if let Some((best, best_distance)) = &mut best_contour {
                if distance < *best_distance {
                    *best_distance = distance;
                    let old = mem::replace(best, (contour, moments, idx));

                    good_contours.push(old.0);
                } else {
                    good_contours.push(contour);
                }
            } else {
                best_contour = Some(((contour, moments, idx), distance));
            }
        }

        if !good_contours.is_empty() {
            imgproc::draw_contours_def(img, &good_contours, -1, (0, 0, 255).into())
                .context("Draw Contours")?;
            if let Some(((contour, moments, idx), _)) = best_contour {
                imgproc::draw_contours_def(img, &self.contours, idx as i32, (0, 255, 0).into())
                    .context("Draw Contours")?;

                let c_x = moments.m10 / moments.m00;
                let c_y = moments.m01 / moments.m00;

                imgproc::draw_marker_def(
                    img,
                    Point::new(poi.x as i32, poi.y as i32),
                    (0, 255, 255).into(),
                )
                .context("Draw POI")?;

                imgproc::draw_marker_def(
                    img,
                    Point::new(c_x as i32, c_y as i32),
                    (255, 0, 0).into(),
                )
                .context("Draw centroid")?;

                let mut rect = imgproc::min_area_rect(&contour).context("Get Rotated Rect")?;
                if rect.size.width > rect.size.height {
                    rect.size.width *= ROI_FACTOR;
                } else {
                    rect.size.height *= ROI_FACTOR;
                }

                let mut points = [Point2f::new(0.0, 0.0); 4];
                rect.points(points.as_mut_slice()).context("Rect points")?;

                imgproc::draw_contours_def(
                    img,
                    &VectorOfVectorOfPoint::from(vec![Vector::from_iter(
                        points
                            .into_iter()
                            .map(|it| Point::new(it.x as i32, it.y as i32)),
                    )]),
                    -1,
                    (255, 0, 0).into(),
                )
                .context("Draw rect")?;

                let mut line = Vec4f::default();
                imgproc::fit_line(&contour, &mut line, imgproc::DIST_L2, 0.0, 0.01, 0.01)
                    .context("Fit Line")?;
                draw_line(
                    img,
                    line,
                    Rect::from_point_size(Point::default(), img_size),
                    (255, 255, 0).into(),
                )
                .context("Draw Centerline")?;
            }
        }

        Ok(img)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // No-op
    }
}

fn draw_line(img: &mut Mat, line: Vec4f, roi: Rect, color: Scalar) -> anyhow::Result<()> {
    let multiplier = roi.width.max(roi.height) as f32;

    let mut start = Point::new(
        (line[2] - multiplier * line[0]) as i32,
        (line[3] - multiplier * line[1]) as i32,
    );

    let mut end = Point::new(
        (line[2] + multiplier * line[0]) as i32,
        (line[3] + multiplier * line[1]) as i32,
    );

    imgproc::clip_line(roi, &mut start, &mut end).context("Clip line")?;
    imgproc::line(img, start, end, color, 1, imgproc::LINE_AA, 0).context("Draw line")?;

    Ok(())
}

fn vector_point_to_slope_intercept(line: Vec4f) -> Vec2f {
    let m = line[1] / line[0];
    let b = -line[2] * m + line[3];

    Vec2f::from_array([m, b])
}

// fn point_in_rect(point: Point2f, rect: RotatedRect) -> bool {}
