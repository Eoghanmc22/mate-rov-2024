use anyhow::{bail, Context};
use bevy::{
    app::{App, Plugin},
    ecs::entity::Entity,
    math::{DVec3, Quat, Vec3, Vec3A},
    prelude::{EntityRef, EntityWorldMut, World},
};
use common::components::{MovementContribution, Orientation, OrientationTarget, Robot, RobotId};
use motor_math::Movement;
use opencv::{
    calib3d,
    core::{self, Scalar, Vector},
    imgproc,
    prelude::*,
    types::{
        VectorOfPoint, VectorOfPoint2d, VectorOfPoint2f, VectorOfPoint3d, VectorOfPoint3f,
        VectorOfVectorOfPoint, VectorOff64,
    },
};

use crate::video_pipelines::{AppPipelineExt, Pipeline, PipelineCallbacks};

pub struct SquarePipelinePlugin;

impl Plugin for SquarePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<SquarePipeline>("Square Detection Pipeline");
    }
}

#[derive(Default)]
pub struct SquarePipeline {
    hsv: Mat,
    mask: Mat,
    mask_tmp: (Mat, Mat, Mat),
    color_mask: Mat,

    contours: VectorOfVectorOfPoint,

    squares: VectorOfVectorOfPoint,
    last_best_square: Option<(f64, f64)>,

    rvec: VectorOff64,
    tvec: VectorOff64,
    // rotation_mat: Mat,
}

impl Pipeline for SquarePipeline {
    // (robot, robot_orientation,)
    type Input = Option<(Entity, Orientation)>;

    fn collect_inputs(world: &World, entity: &EntityRef) -> Self::Input {
        let robot_id = entity.get::<RobotId>()?;

        let robot = world.iter_entities().find(|entity| {
            entity.contains::<Robot>() && entity.get::<RobotId>() == Some(robot_id)
        })?;

        let &orientation = robot.get::<Orientation>()?;

        Some((robot.id(), orientation))
    }

    fn process<'b, 'a: 'b>(
        &'a mut self,
        cmds: &mut PipelineCallbacks,
        data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b mut Mat> {
        let Some((robot, orientation)) = *data else {
            return Ok(img);
        };

        // Use HSV to better differentiate colors
        imgproc::cvt_color_def(img, &mut self.hsv, imgproc::COLOR_BGR2HSV)
            .context("Convert to HSV")?;

        // Bounds for what counts as red
        let lower_red_1: Scalar = (0, 30, 100).into();
        let upper_red_1: Scalar = (15, 255, 255).into();
        let lower_red_2: Scalar = (160, 30, 100).into();
        let upper_red_2: Scalar = (180, 255, 255).into();

        // Create mask containing everything thats red
        core::in_range(&self.hsv, &lower_red_1, &upper_red_1, &mut self.mask_tmp.0)
            .context("Mask 1")?;
        core::in_range(&self.hsv, &lower_red_2, &upper_red_2, &mut self.mask_tmp.1)
            .context("Mask 2")?;
        core::add_def(&self.mask_tmp.0, &self.mask_tmp.1, &mut self.mask).context("Merge masks")?;
        // core::add_def(&self.mask_tmp.0, &self.mask_tmp.1, &mut self.mask_tmp.2)
        //     .context("Merge masks")?;

        // let kernel = Mat::ones(15, 15, core::CV_8U).context("kenel")?;
        // imgproc::morphology_ex_def(
        //     &self.mask_tmp.2,
        //     &mut self.mask,
        //     imgproc::MORPH_OPEN,
        //     &kernel,
        // )
        // .context("Morphology")?;

        // self.color_mask = Mat::default();
        // core::bitwise_and(img, img, &mut self.color_mask, &self.mask).context("Color mask")?;
        // return Ok(&mut self.color_mask);

        // Find contours in mask
        self.contours.clear();
        imgproc::find_contours_def(
            &self.mask,
            &mut self.contours,
            imgproc::RETR_LIST,
            // TODO: Are the other approximation modes better?
            imgproc::CHAIN_APPROX_SIMPLE,
        )
        .context("Find contours")?;

        // Find a subset of controus that are 4 point convex polygons
        self.squares.clear();
        for contour in &self.contours {
            let epsilon = 0.02 * imgproc::arc_length(&contour, true).context("Find arc length")?;

            let mut approx = VectorOfPoint::default();
            imgproc::approx_poly_dp(&contour, &mut approx, epsilon, true)
                .context("Approximate polygon")?;

            if approx.len() == 4 {
                let is_convex = imgproc::is_contour_convex(&approx).context("Is convex")?;
                let area = imgproc::contour_area_def(&approx).context("Area")?;

                // TODO: Determine good threshold
                if is_convex && area > 750.0 {
                    self.squares.push(approx);
                }
            }
        }

        // Choose best square based on
        // - Size
        // - Proximity to best square in previous frame
        let mut best: Option<(f64, (f64, f64), VectorOfPoint)> = None;
        for square in &self.squares {
            let moments = imgproc::moments_def(&square).context("Moments")?;

            let c_x = moments.m10 / moments.m00;
            let c_y = moments.m01 / moments.m00;

            let mut score = 5.0 * moments.m00;
            if let Some(last_best_square) = self.last_best_square {
                // TODO: This needs to be a similar magnitude to area
                score -= (c_x - last_best_square.0).powi(2) + (c_y - last_best_square.1).powi(2);
            }

            if let Some((best_score, ..)) = best {
                if score > best_score {
                    best = Some((score, (c_x, c_y), square));
                }
            } else {
                best = Some((score, (c_x, c_y), square));
            }
        }

        if let Some((_, position, square)) = best {
            self.last_best_square = Some(position);

            let contours: VectorOfVectorOfPoint = vec![square.clone()].into();
            imgproc::draw_contours_def(img, &contours, -1, (0, 255, 0).into())
                .context("Draw Contours")?;

            let square_size = 0.15;

            let obj_points: VectorOfPoint3f = vec![
                (-square_size / 2.0, square_size / 2.0, 0.0).into(),
                (square_size / 2.0, square_size / 2.0, 0.0).into(),
                (square_size / 2.0, -square_size / 2.0, 0.0).into(),
                (-square_size / 2.0, -square_size / 2.0, 0.0).into(),
            ]
            .into();

            let img_points: VectorOfPoint2f = square.iter().flat_map(|it| it.to::<f32>()).collect();

            // TODO: Need values for these
            let camera_matrix = Mat::from_slice_rows_cols(
                &[
                    1.28191219e+03,
                    0.00000000e+00,
                    1.01414124e+03,
                    0.00000000e+00,
                    1.28020562e+03,
                    5.30598083e+02,
                    0.00000000e+00,
                    0.00000000e+00,
                    1.00000000e+00,
                ],
                3,
                3,
            )
            .context("Create mock camera matrix")?;
            let dist_coeffs = VectorOff64::from_slice(&[
                -4.01928524e-01,
                2.05847758e-01,
                -1.51617786e-04,
                7.81120105e-04,
                -5.77244616e-02,
            ]);

            println!("square: {square:?}");
            println!("obj: {obj_points:.2?}");

            let success = calib3d::solve_pnp(
                &obj_points,
                &img_points,
                &camera_matrix,
                &dist_coeffs,
                &mut self.rvec,
                &mut self.tvec,
                false,
                calib3d::SOLVEPNP_IPPE_SQUARE,
            )
            .context("Solve PnP")?;

            if !success {
                bail!("Bad PnP");
            }

            // Draw axis
            {
                let points: VectorOfPoint3f = vec![
                    (-square_size / 2.0, square_size / 2.0, 0.0).into(),
                    (square_size / 2.0, square_size / 2.0, 0.0).into(),
                    (square_size / 2.0, -square_size / 2.0, 0.0).into(),
                    (-square_size / 2.0, -square_size / 2.0, 0.0).into(),
                    (0.0, 0.0, 0.0).into(),
                    (0.0, 0.0, square_size).into(),
                ]
                .into();

                let mut projected_points: VectorOfPoint2f = Vector::default();
                calib3d::project_points_def(
                    &points,
                    &self.rvec,
                    &self.tvec,
                    &camera_matrix,
                    &dist_coeffs,
                    &mut projected_points,
                )
                .context("Project axis points")?;

                imgproc::line_def(
                    img,
                    projected_points
                        .get(0)?
                        .to()
                        .context("Cast projected point")?,
                    projected_points
                        .get(2)?
                        .to()
                        .context("Cast projected point")?,
                    (255, 0, 0).into(),
                )
                .context("Line")?;
                imgproc::line_def(
                    img,
                    projected_points
                        .get(1)?
                        .to()
                        .context("Cast projected point")?,
                    projected_points
                        .get(3)?
                        .to()
                        .context("Cast projected point")?,
                    (255, 0, 0).into(),
                )
                .context("Line")?;
                imgproc::line_def(
                    img,
                    projected_points
                        .get(4)?
                        .to()
                        .context("Cast projected point")?,
                    projected_points
                        .get(5)?
                        .to()
                        .context("Cast projected point")?,
                    (255, 255, 255).into(),
                )
                .context("Line")?;
            }

            // calib3d::rodrigues_def(&self.rvec, &mut self.rotation_mat).context("Rodrigues")?;
            //
            // let position = self
            //     .rotation_mat
            //     .inv_def()
            //     .context("Invert")?
            //     .mul_matexpr_def(&core::negate(&self.tvec).context("Negate")?)
            //     .context("Mul")?
            //     .to_mat()
            //     .context("To mat")?;

            let position_delta = DVec3::new(
                self.tvec.get(0).context("Read tvec X")?,
                self.tvec.get(2).context("Read tvec Z")?,
                self.tvec.get(1).context("Read tvec Y")?,
            )
            .as_vec3();

            println!("delta: {position_delta:.2?}");

            // Movement
            //
            // GOAL:
            // - Point robot towards target
            // - Move robot planar untill it is above target
            // TODO:
            // - Once above target, lower robot onto target

            let robot_orientation = orientation.0;
            // TODO: Scale down the adjustment?
            let new_orientation_target =
                robot_orientation * Quat::from_rotation_arc(position_delta.normalize(), Vec3::Y);

            let speed = 10.0;
            let max_speed = 30.0;

            let mut movement_world = robot_orientation.inverse() * position_delta;
            movement_world.z = 0.0;
            let movement_planar =
                (robot_orientation * movement_world * speed).clamp_length_max(max_speed);

            cmds.pipeline(move |mut entity| {
                entity.insert(MovementContribution(Movement {
                    force: movement_planar.into(),
                    torque: Vec3A::ZERO,
                }));

                entity.world_scope(|world| {
                    let Some(mut robot) = world.get_entity_mut(robot) else {
                        return;
                    };

                    robot.insert(OrientationTarget(new_orientation_target));
                });
            });
        } else {
            // TODO: Cancel movement and log? or maybe assume the trajectory didnt change
            cmds.pipeline(move |mut entity| {
                entity.insert(MovementContribution(Movement {
                    force: Vec3A::ZERO,
                    torque: Vec3A::ZERO,
                }));
            });
        }

        Ok(img)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // No-op
    }
}
