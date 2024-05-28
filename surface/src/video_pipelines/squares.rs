use anyhow::{bail, Context};
use bevy::{
    app::{App, Plugin},
    ecs::entity::Entity,
    math::{DVec3, Quat, Vec3, Vec3A},
    prelude::{EntityRef, EntityWorldMut, World},
};
use common::{
    components::{
        Depth, DepthTarget, MovementContribution, Orientation, OrientationTarget, Robot, RobotId,
        ServoContribution, ServoTargets,
    },
    types::units::Meters,
};
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
use tracing::error;

use crate::video_pipelines::{AppPipelineExt, Pipeline, PipelineCallbacks};

// Autonomous pipeline for brain coral transplantation
pub struct SquarePipelinePlugin;

impl Plugin for SquarePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.register_video_pipeline::<SquareTrackingPipeline>("Square Tracking Pipeline");
    }
}

// Stores internal state necessry for tracking the target
#[derive(Default)]
pub struct SquareTrackingPipeline {
    // Track what the ROV is currently trying to do
    state: InternalState,

    // Image in HSV
    hsv: Mat,
    // Mask of all "red" pixels
    mask: Mat,
    // Store partial mask computations to avoid allocation costs
    mask_tmp: (Mat, Mat, Mat),
    // Image containing only pixels clasified as red
    color_mask: Mat,

    // List of contours in the mask
    contours: VectorOfVectorOfPoint,

    // List of contours that look like squares
    squares: VectorOfVectorOfPoint,
    // The position of the center of shape considered to be the best canidate for being the target
    last_best_square: Option<(f64, f64)>,

    // Computed rotation relative to the square
    rvec: VectorOff64,
    // Computed translation relative to the square
    tvec: VectorOff64,
    // rotation_mat: Mat,
}

// State Machiene for target following pipeline
#[derive(Default)]
enum InternalState {
    #[default]
    MoveAboveTarget,
    LowerDepth,
    ReleasePayload,
}

impl Pipeline for SquareTrackingPipeline {
    // (robot, robot_orientation,)
    type Input = Option<(Entity, Orientation, Depth, ServoTargets)>;

    // Extracts the necessary data from the ECS world
    // Runs on the main thread
    fn collect_inputs(world: &World, entity: &EntityRef) -> Self::Input {
        // Get id of attached robot
        let robot_id = entity.get::<RobotId>()?;

        // Find which entity is a robot and has that id
        let robot = world.iter_entities().find(|entity| {
            entity.contains::<Robot>() && entity.get::<RobotId>() == Some(robot_id)
        })?;

        // Read the robot's orientation from the IMU
        let &orientation = robot.get::<Orientation>()?;

        // Read the robot's depth from the pressure sensor
        let &depth = robot.get::<Depth>()?;

        // Read the target positions of the robot's servos
        let servos = robot.get::<ServoTargets>()?.clone();

        Some((robot.id(), orientation, depth, servos))
    }

    // Process the latest frame from the camera
    // Runs async
    fn process<'b, 'a: 'b>(
        &'a mut self,
        cmds: &mut PipelineCallbacks,
        data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b mut Mat> {
        // Make sure we have know the robot orientation
        let Some((robot, orientation, depth, ref servos)) = *data else {
            return Ok(img);
        };

        // Try to run the image processing pipeline
        let res: Result<_, anyhow::Error> = try {
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
            core::add_def(&self.mask_tmp.0, &self.mask_tmp.1, &mut self.mask)
                .context("Merge masks")?;
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
                // Define the required accuracy
                let epsilon =
                    0.02 * imgproc::arc_length(&contour, true).context("Find arc length")?;

                // Approximate a polygon that is a similar shape to the contour
                let mut approx = VectorOfPoint::default();
                imgproc::approx_poly_dp(&contour, &mut approx, epsilon, true)
                    .context("Approximate polygon")?;

                // If the polygon has 4 fourners
                // Run some final checks
                if approx.len() == 4 {
                    let is_convex = imgproc::is_contour_convex(&approx).context("Is convex")?;
                    let area = imgproc::contour_area_def(&approx).context("Area")?;

                    // TODO: Determine good threshold
                    if is_convex && area > 750.0 {
                        // Its square enough to be considered a canidate
                        self.squares.push(approx);
                    }
                }
            }

            // Choose best square based on
            // - Size
            // - Proximity to best square in previous frame (improves temporal consistancy)
            let mut best: Option<(f64, (f64, f64), VectorOfPoint)> = None;
            for square in &self.squares {
                let moments = imgproc::moments_def(&square).context("Moments")?;

                let c_x = moments.m10 / moments.m00;
                let c_y = moments.m01 / moments.m00;

                let mut score = 5.0 * moments.m00;
                if let Some(last_best_square) = self.last_best_square {
                    // TODO: This needs to be a similar magnitude to area
                    score -=
                        (c_x - last_best_square.0).powi(2) + (c_y - last_best_square.1).powi(2);
                }

                if let Some((best_score, ..)) = best {
                    if score > best_score {
                        best = Some((score, (c_x, c_y), square));
                    }
                } else {
                    best = Some((score, (c_x, c_y), square));
                }
            }

            // If a best canidate was found
            if let Some((_, position, square)) = best {
                // Store it for future reference
                self.last_best_square = Some(position);

                // Draw it on screen
                let contours: VectorOfVectorOfPoint = vec![square.clone()].into();
                imgproc::draw_contours_def(img, &contours, -1, (0, 255, 0).into())
                    .context("Draw Contours")?;

                // Actual size of the target in meters
                let square_size = 0.15;

                // 3D points of the four corners of the target
                let obj_points: VectorOfPoint3f = vec![
                    (-square_size / 2.0, square_size / 2.0, 0.0).into(),
                    (square_size / 2.0, square_size / 2.0, 0.0).into(),
                    (square_size / 2.0, -square_size / 2.0, 0.0).into(),
                    (-square_size / 2.0, -square_size / 2.0, 0.0).into(),
                ]
                .into();

                let img_points: VectorOfPoint2f =
                    square.iter().flat_map(|it| it.to::<f32>()).collect();

                // Tempoary hard coded camera martix
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
                .context("Create temp camera matrix")?;

                // Tempoary hard coded distortion coefficients
                let dist_coeffs = VectorOff64::from_slice(&[
                    -4.01928524e-01,
                    2.05847758e-01,
                    -1.51617786e-04,
                    7.81120105e-04,
                    -5.77244616e-02,
                ]);

                println!("square: {square:?}");
                println!("obj: {obj_points:.2?}");

                // Use the known dimensions of the square to determine where it is in 3D space
                // relative to the ROV
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

                // Make sure it actually worked
                if !success {
                    bail!("Bad PnP");
                }

                // Draw 3D axis on screen
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
            }
        };

        // Work around the fact that if we return the error like normal it will skip presenting the
        // processed frame. Errors here are only handeled by the callee logging them anyways
        if let Err(err) = res {
            error!("Square tracking pipeline error: {err:?}");
        }

        // Determine position relative to target in 3D
        // Unit is meters
        // Need to flip Y and Z to align camera space with the ROV's movement space
        let position_delta = DVec3::new(
            self.tvec.get(0).context("Read tvec X")?,
            self.tvec.get(2).context("Read tvec Z")?,
            self.tvec.get(1).context("Read tvec Y")?,
        )
        .as_vec3();

        println!("delta: {position_delta:.2?}");

        // Movement
        //
        // PLAN:
        // - Point robot towards target
        // - Move robot planar untill it is above target
        // - Once above target, lower robot onto target
        // - Open claw to release payload onto target

        // Need to always keep the target in the center of the camera's view
        // Calcualte the robot orientation necessary for that
        let robot_orientation = orientation.0;
        // TODO: Scale down the adjustment?
        // TODO: Implement smoothening
        let new_orientation_target =
            robot_orientation * Quat::from_rotation_arc(position_delta.normalize(), Vec3::Y);

        // Speed constants
        let speed = 10.0;
        let max_speed = 30.0;

        // Need to try to get the planar position of the ROV to be directly above the target
        // Compute what correction is necessary for that
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

        // Update state machine
        match &self.state {
            // Try to position the robot directly above of target
            InternalState::MoveAboveTarget => {
                // Set correct depth initial depth
                cmds.world(move |world| {
                    let Some(mut robot) = world.get_entity_mut(robot) else {
                        return;
                    };

                    robot.insert(DepthTarget(Meters(0.6)));
                });

                // If ROV is within 7cm planar distance of the target
                // begin decending onto the target
                if movement_world.length_squared() < 0.07 * 0.07 {
                    self.state = InternalState::LowerDepth;
                }
            }
            InternalState::LowerDepth => {
                // We assume the ROV is facing downward by now
                let remaing_depth = position_delta.y - 0.1;

                // Lower depth slowly by continually setting the depth target this distance lower
                // than the current depth or by `remaining_depth` which ever is less
                let depth_target_delta = 0.07f32;
                let new_depth_target = depth.0.depth.0 + depth_target_delta.min(remaing_depth);

                // Send new depth target to robot
                cmds.world(move |world| {
                    let Some(mut robot) = world.get_entity_mut(robot) else {
                        return;
                    };

                    robot.insert(DepthTarget(Meters(new_depth_target)));
                });

                // At depth target, release the payload.
                if remaing_depth.abs() < 0.03 {
                    self.state = InternalState::ReleasePayload;
                }
            }
            InternalState::ReleasePayload => {
                // Slowly open claw
                cmds.pipeline(move |mut entity| {
                    entity.insert(ServoContribution([("Claw1".into(), -0.1)].into()));
                });

                // If claw is open, end the pipeline
                if servos.0.get("Claw1").iter().any(|&&val| val < -0.8) {
                    cmds.should_end();
                }
            }
        }

        // Present processed camera image to the screen
        Ok(img)
    }

    fn cleanup(_entity_world: &mut EntityWorldMut) {
        // Pipeline entity is automatically despawned
        // No-op
    }
}
