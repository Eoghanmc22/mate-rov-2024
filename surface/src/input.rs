use std::{borrow::Cow, mem};

use ahash::HashSet;
use bevy::{
    math::{vec3a, Vec3A},
    prelude::*,
};
use common::{
    bundles::MovementContributionBundle,
    components::{
        Armed, Depth, DepthTarget, MovementAxisMaximums, MovementContribution, Orientation,
        OrientationTarget, Robot, RobotId, ServoContribution, Servos,
    },
    ecs_sync::{NetId, Replicate},
    events::ResetServo,
    types::units::Meters,
};
use egui::TextBuffer;
use leafwing_input_manager::{
    action_state::ActionState, axislike::SingleAxis, input_map::InputMap,
    plugin::InputManagerPlugin, Actionlike, InputManagerBundle,
};
use motor_math::{solve::reverse::Axis, Movement};

// TODO(low): Handle multiple gamepads better
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<InputInterpolation>()
            .add_plugins(InputManagerPlugin::<Action>::default())
            .add_systems(
                Update,
                (
                    attach_to_new_robots,
                    handle_disconnected_robots,
                    movement,
                    arm,
                    depth_hold,
                    leveling,
                    trim_orientation,
                    trim_depth,
                    servos,
                    robot_mode,
                    switch_pitch_roll,
                ),
            );
    }
}

#[derive(Component, Debug, Clone, Default, Reflect)]
pub struct SelectedServo {
    pub servo: Option<Cow<'static, str>>,
}

#[derive(Component, Debug, Clone, Copy, Reflect, PartialEq)]
pub struct InputInterpolation {
    depth_mps: f32,
    trim_dps: f32,
    servo_rate: f32,

    power: f32,
    scale: f32,
}

impl InputInterpolation {
    pub fn interpolate_input(&self, input: f32) -> f32 {
        input.powf(self.power).copysign(input) * self.scale
    }

    pub const fn normal() -> Self {
        Self {
            depth_mps: 0.3,
            trim_dps: 60.0,
            servo_rate: 5.0,
            power: 3.0,
            scale: 0.8,
        }
    }

    pub const fn precision() -> Self {
        Self {
            depth_mps: 0.1,
            trim_dps: 60.0,
            servo_rate: 4.0,
            power: 3.0,
            scale: 0.3,
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum Action {
    Arm,
    Disarm,

    // IncreaseGain,
    // DecreaseGain,
    // ResetGain,
    ToggleDepthHold,
    ToggleLeveling(LevelingType),

    ToggleRobotMode,

    Surge,
    SurgeInverted,
    Heave,
    HeaveInverted,
    Sway,
    SwayInverted,

    Pitch,
    PitchInverted,
    Roll,
    RollInverted,
    Yaw,
    YawInverted,
    // HoldAxis,
    Servo,
    ServoCenter,
    ServoInverted,
    SwitchServo,
    SwitchServoInverted,
    SelectImportantServo,

    SwitchPitchRoll,
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect, Default)]
pub enum LevelingType {
    #[default]
    Upright,
    Inverted,
}

#[derive(Component)]
pub struct InputMarker;

fn attach_to_new_robots(mut cmds: Commands, new_robots: Query<(&NetId, &Name), Added<Robot>>) {
    for (robot, name) in &new_robots {
        let mut input_map = InputMap::default();

        input_map.insert(Action::Disarm, GamepadButtonType::Select);
        input_map.insert(Action::Arm, GamepadButtonType::Start);

        input_map.insert(Action::Disarm, KeyCode::Space);
        input_map.insert(Action::Arm, KeyCode::Enter);

        input_map.insert(
            Action::ToggleLeveling(LevelingType::Upright),
            GamepadButtonType::North,
        );
        input_map.insert(
            Action::ToggleLeveling(LevelingType::Inverted),
            GamepadButtonType::South,
        );
        input_map.insert(Action::ToggleDepthHold, GamepadButtonType::East);
        // input_map.insert(Action::ToggleDepthHold, GamepadButtonType::North);
        // input_map.insert(Action::ToggleDepthHold, GamepadButtonType::South);
        input_map.insert(Action::SwitchPitchRoll, GamepadButtonType::West);

        input_map.insert(
            Action::Yaw,
            SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.05),
        );
        input_map.insert(
            Action::Surge,
            SingleAxis::symmetric(GamepadAxisType::LeftStickY, 0.05),
        );

        input_map.insert(
            Action::Sway,
            SingleAxis::symmetric(GamepadAxisType::RightStickX, 0.05),
        );
        input_map.insert(
            Action::Heave,
            SingleAxis::symmetric(GamepadAxisType::RightStickY, 0.05),
        );

        input_map.insert(Action::Servo, GamepadButtonType::RightTrigger);
        input_map.insert(Action::ServoInverted, GamepadButtonType::LeftTrigger);
        // input_map.insert(Action::Pitch, GamepadButtonType::RightTrigger);
        // input_map.insert(Action::PitchInverted, GamepadButtonType::LeftTrigger);

        // input_map.insert(Action::Roll, GamepadButtonType::RightTrigger2);
        // input_map.insert(Action::RollInverted, GamepadButtonType::LeftTrigger2);
        input_map.insert(Action::Pitch, GamepadButtonType::RightTrigger2);
        input_map.insert(Action::PitchInverted, GamepadButtonType::LeftTrigger2);

        input_map.insert(Action::ServoCenter, GamepadButtonType::DPadUp);
        // input_map.insert(Action::Servo, GamepadButtonType::DPadRight);
        // input_map.insert(Action::ServoInverted, GamepadButtonType::DPadLeft);
        input_map.insert(Action::SwitchServo, GamepadButtonType::DPadRight);
        input_map.insert(Action::SwitchServoInverted, GamepadButtonType::DPadLeft);
        input_map.insert(Action::SelectImportantServo, GamepadButtonType::DPadDown);

        input_map.insert(Action::ToggleRobotMode, GamepadButtonType::Mode);
        input_map.insert(Action::ToggleRobotMode, GamepadButtonType::West);

        // input_map.insert(
        //     Action::Yaw,
        //     SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.05),
        // );
        // input_map.insert(
        //     Action::Pitch,
        //     SingleAxis::symmetric(GamepadAxisType::LeftStickY, 0.05),
        // );
        //
        // input_map.insert(
        //     Action::Sway,
        //     SingleAxis::symmetric(GamepadAxisType::RightStickX, 0.05),
        // );
        // input_map.insert(
        //     Action::Heave,
        //     SingleAxis::symmetric(GamepadAxisType::RightStickY, 0.05),
        // );
        //
        // input_map.insert(Action::Roll, GamepadButtonType::RightTrigger);
        // input_map.insert(Action::RollInverted, GamepadButtonType::LeftTrigger);
        //
        // input_map.insert(Action::Surge, GamepadButtonType::RightTrigger2);
        // input_map.insert(Action::SurgeInverted, GamepadButtonType::LeftTrigger2);

        cmds.spawn((
            SelectedServo::default(),
            InputManagerBundle::<Action> {
                // Stores "which actions are currently pressed"
                action_state: ActionState::default(),
                // Describes how to convert from player inputs into those actions
                input_map,
            },
            MovementContributionBundle {
                name: Name::new(format!("HID {name}")),
                contribution: MovementContribution(Movement::default()),
                robot: RobotId(*robot),
            },
            ServoContribution(Default::default()),
            InputInterpolation::normal(),
            InputMarker,
            Replicate,
        ));
    }
}

fn handle_disconnected_robots(
    mut cmds: Commands,
    robots: Query<&NetId, With<Robot>>,
    inputs: Query<(Entity, &RobotId), With<InputMarker>>,
    mut removed_robots: RemovedComponents<Robot>,
) {
    for _robot in removed_robots.read() {
        let robots: HashSet<NetId> = robots.iter().copied().collect();

        inputs
            .iter()
            .filter(|(_, &RobotId(robot))| !robots.contains(&robot))
            .for_each(|(entity, _)| cmds.entity(entity).despawn());
    }
}

// TODO(mid): Remap sticks to square. See http://theinstructionlimit.com/squaring-the-thumbsticks
fn movement(
    mut cmds: Commands,
    inputs: Query<(Entity, &RobotId, &ActionState<Action>, &InputInterpolation), With<InputMarker>>,
    robots: Query<
        (
            &MovementAxisMaximums,
            Option<&DepthTarget>,
            Option<&Orientation>,
            Option<&OrientationTarget>,
            &RobotId,
        ),
        With<Robot>,
    >,
) {
    for (entity, robot, action_state, interpolation) in &inputs {
        let Some((
            MovementAxisMaximums(maximums),
            depth_target,
            orientation,
            orientation_target,
            _,
        )) = robots
            .iter()
            .find(|(_, _, _, _, robot_id)| robot_id.0 == robot.0)
        else {
            error!("Could not find robot for input");

            continue;
        };

        let x = interpolation.interpolate_input(
            action_state.value(&Action::Sway) - action_state.value(&Action::SwayInverted),
        ) * maximums[&Axis::X].0;
        let y = interpolation.interpolate_input(
            action_state.value(&Action::Surge) - action_state.value(&Action::SurgeInverted),
        ) * maximums[&Axis::Y].0;
        let z = interpolation.interpolate_input(
            action_state.value(&Action::Heave) - action_state.value(&Action::HeaveInverted),
        ) * maximums[&Axis::Z].0;

        let x_rot = interpolation.interpolate_input(
            action_state.value(&Action::Pitch) - action_state.value(&Action::PitchInverted),
        ) * maximums[&Axis::XRot].0;
        let y_rot = interpolation.interpolate_input(
            action_state.value(&Action::Roll) - action_state.value(&Action::RollInverted),
        ) * maximums[&Axis::YRot].0;
        let z_rot = interpolation.interpolate_input(
            -(action_state.value(&Action::Yaw) - action_state.value(&Action::YawInverted)),
        ) * maximums[&Axis::ZRot].0;

        let force = if depth_target.is_some() {
            if let Some(orientation) = orientation {
                let mut yaw = orientation.0;
                if yaw.z.abs() * yaw.z.abs() + yaw.w.abs() * yaw.w.abs() > 0.1 {
                    yaw.x = 0.0;
                    yaw.y = 0.0;
                    yaw = yaw.normalize()
                } else {
                    yaw *= Quat::from_rotation_y(180f32.to_radians());
                    yaw.x = 0.0;
                    yaw.y = 0.0;
                    yaw = -yaw.normalize();
                    // yaw *= Quat::from_rotation_y(180f32.to_radians()).inverse();
                }

                let world_force = yaw * vec3a(x, y, 0.0);

                orientation.0.inverse() * world_force
            } else {
                vec3a(x, y, 0.0)
            }
        } else {
            vec3a(x, y, z)
        };

        let torque = if orientation_target.is_some() {
            Vec3A::ZERO
        } else {
            vec3a(x_rot, y_rot, z_rot)
        };

        let movement = Movement { force, torque };

        cmds.entity(entity).insert(MovementContribution(movement));
    }
}

fn arm(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<(Entity, &RobotId), With<Robot>>,
) {
    for (robot, action_state) in &inputs {
        let disarm = action_state.just_pressed(&Action::Disarm);
        let arm = action_state.just_pressed(&Action::Arm);

        let robot = robots.iter().find(|&(_, other_robot)| robot == other_robot);

        if let Some((robot, _)) = robot {
            if disarm {
                info!("Disarming");
                cmds.entity(robot).insert(Armed::Disarmed);
            } else if arm {
                info!("Arming");
                cmds.entity(robot).insert(Armed::Armed);
            }
        } else if arm || disarm {
            warn!("No ROV attached");
        }
    }
}

fn depth_hold(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<(Entity, &Depth, Option<&DepthTarget>, &RobotId), With<Robot>>,
) {
    for (robot, action_state) in &inputs {
        let toggle = action_state.just_pressed(&Action::ToggleDepthHold);

        let robot = robots
            .iter()
            .find(|&(_, _, _, other_robot)| robot == other_robot);

        if let Some((robot, depth, depth_target, _)) = robot {
            if toggle {
                match depth_target {
                    Some(_) => {
                        info!("Clear Depth Hold");
                        cmds.entity(robot).remove::<DepthTarget>();
                    }
                    None => {
                        let depth = depth.0.depth;

                        info!("Set Depth Hold: {:.2}", depth);
                        cmds.entity(robot).insert(DepthTarget(depth));
                    }
                }
            }
        } else if toggle {
            warn!("No ROV attached");
        }
    }
}

fn leveling(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>), With<InputMarker>>,
    robots: Query<(Entity, &Orientation, Option<&OrientationTarget>, &RobotId), With<Robot>>,
) {
    for (robot, action_state) in &inputs {
        let toggle_upright =
            action_state.just_pressed(&Action::ToggleLeveling(LevelingType::Upright));
        let toggle_inverted =
            action_state.just_pressed(&Action::ToggleLeveling(LevelingType::Inverted));

        let robot = robots
            .iter()
            .find(|&(_, _, _, other_robot)| robot == other_robot);

        if let Some((robot, orientation, orientation_target, _)) = robot {
            if toggle_upright || toggle_inverted {
                let mut new_target = orientation.0;

                // Only keep yaw component
                new_target.x = 0.0;
                new_target.y = 0.0;
                let new_target = new_target.normalize();

                // Flip if inverted is selected
                let new_target = if toggle_upright {
                    new_target
                } else {
                    new_target * Quat::from_rotation_y(180f32.to_radians())
                };

                match orientation_target {
                    // FIXME: Make switching from upright to inverted easier
                    Some(_old_target) => {
                        //if old_target.0 == new_target => {
                        info!("Clear Leveling");
                        cmds.entity(robot).remove::<OrientationTarget>();
                    }
                    _ => {
                        if toggle_upright {
                            info!("Set Level Upright");
                        } else {
                            info!("Set Level Inverted");
                        }

                        cmds.entity(robot).insert(OrientationTarget(new_target));
                    }
                }
            }
        } else if toggle_upright || toggle_inverted {
            warn!("No ROV attached");
        }
    }
}

fn trim_orientation(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>, &InputInterpolation), With<InputMarker>>,
    robots: Query<(Entity, &Orientation, Option<&OrientationTarget>, &RobotId), With<Robot>>,
    time: Res<Time<Real>>,
) {
    for (robot, action_state, interpolation) in &inputs {
        let pitch = interpolation.interpolate_input(
            action_state.value(&Action::Pitch) - action_state.value(&Action::PitchInverted),
        );
        let roll = interpolation.interpolate_input(
            action_state.value(&Action::Roll) - action_state.value(&Action::RollInverted),
        );
        let yaw = interpolation.interpolate_input(
            -(action_state.value(&Action::Yaw) - action_state.value(&Action::YawInverted)),
        );

        let robot = robots
            .iter()
            .find(|&(_, _, _, other_robot)| robot == other_robot);

        if let Some((robot, orientation, orientation_target, _)) = robot {
            let Some(&OrientationTarget(mut orientation_target)) = orientation_target else {
                continue;
            };

            if pitch.abs() >= 0.05 {
                let input = pitch * interpolation.trim_dps * time.delta_seconds();
                orientation_target = orientation_target * Quat::from_rotation_x(input.to_radians());
            }

            if roll.abs() >= 0.05 {
                let input = roll * interpolation.trim_dps * time.delta_seconds();
                orientation_target = orientation_target * Quat::from_rotation_y(input.to_radians());
            }

            if yaw.abs() >= 0.05 {
                let input = yaw * interpolation.trim_dps * time.delta_seconds();
                orientation_target = Quat::from_rotation_z(input.to_radians()) * orientation_target;
            }

            if pitch != 0.0 || roll != 0.0 || yaw != 0.0 {
                cmds.entity(robot)
                    .insert(OrientationTarget(orientation_target));
            }
        } else if pitch != 0.0 || roll != 0.0 || yaw != 0.0 {
            warn!("No ROV attached");
        }
    }
}

fn trim_depth(
    mut cmds: Commands,
    inputs: Query<(&RobotId, &ActionState<Action>, &InputInterpolation), With<InputMarker>>,
    robots: Query<(Entity, Option<&DepthTarget>, Option<&Orientation>, &RobotId), With<Robot>>,
    time: Res<Time<Real>>,
) {
    for (robot, action_state, interpolation) in &inputs {
        let z = interpolation.interpolate_input(
            action_state.value(&Action::Heave) - action_state.value(&Action::HeaveInverted),
        );

        let robot = robots
            .iter()
            .find(|&(_, _, _, other_robot)| robot == other_robot);

        if let Some((robot, depth_target, orientation, _)) = robot {
            let Some(&DepthTarget(Meters(mut depth_target))) = depth_target else {
                continue;
            };

            if z != 0.0 {
                let mut input = z * interpolation.depth_mps * time.delta_seconds();

                if let Some(orientation) = orientation {
                    input *= (orientation.0 * Vec3A::Z).z.signum();
                }

                depth_target -= input;
                if depth_target < 0.0 {
                    depth_target = 0.0;
                }
                cmds.entity(robot).insert(DepthTarget(depth_target.into()));
            }
        } else if z != 0.0 {
            warn!("No ROV attached");
        }
    }
}

fn servos(
    mut cmds: Commands,
    mut inputs: Query<
        (
            Entity,
            &RobotId,
            &ActionState<Action>,
            &InputInterpolation,
            // TODO: Make this not mut?
            &mut SelectedServo,
        ),
        With<InputMarker>,
    >,
    mut writer: EventWriter<ResetServo>,
    robots: Query<(&Servos, &RobotId), With<Robot>>,
) {
    for (entity, robot, action_state, interpolation, mut selected_servo) in &mut inputs {
        let center = action_state.just_pressed(&Action::ServoCenter);
        let switch = action_state.just_pressed(&Action::SwitchServo);
        let switch_inverted = action_state.just_pressed(&Action::SwitchServoInverted);
        let select_important = action_state.just_pressed(&Action::SelectImportantServo);
        let input = action_state.value(&Action::Servo) - action_state.value(&Action::ServoInverted);

        let robot = robots.iter().find(|&(_, other_robot)| robot == other_robot);

        if let Some((servos, _)) = robot {
            let offset = if switch {
                1
            } else {
                servos.servos.len().saturating_sub(1)
            };

            if select_important {
                if selected_servo.servo.as_ref().map(|it| it.as_str()) != Some("Claw1") {
                    if servos.servos.iter().any(|it| it.as_str() == "Claw1") {
                        selected_servo.servo = Some("Claw1".into());
                    }
                } else if servos
                    .servos
                    .iter()
                    .any(|it| it.as_str() == "FrontCameraRotate")
                {
                    selected_servo.servo = Some("FrontCameraRotate".into());
                }
            } else if (switch || switch_inverted || selected_servo.servo.is_none())
                && !servos.servos.is_empty()
            {
                let idx = servos
                    .servos
                    .iter()
                    .position(|it| {
                        Some(it.as_str()) == selected_servo.servo.as_ref().map(|it| it.as_str())
                    })
                    .map(|it| (it + offset) % servos.servos.len())
                    .unwrap_or(0);

                selected_servo.servo = Some(servos.servos[idx].clone());
            }

            if let Some(servo) = &selected_servo.servo {
                if center {
                    writer.send(ResetServo(servo.clone()));
                }

                let movement = input * interpolation.servo_rate;

                cmds.entity(entity).insert(ServoContribution(
                    vec![(servo.clone(), movement)].into_iter().collect(),
                ));
            }
        }
    }
}

fn robot_mode(
    mut inputs: Query<(&ActionState<Action>, &mut InputInterpolation), With<InputMarker>>,
) {
    for (action_state, mut interpolation) in &mut inputs {
        let toggle = action_state.just_pressed(&Action::ToggleRobotMode);

        if toggle {
            if *interpolation == InputInterpolation::normal() {
                *interpolation = InputInterpolation::precision()
            } else {
                *interpolation = InputInterpolation::normal()
            }
        }
    }
}

fn switch_pitch_roll(
    mut inputs: Query<(&ActionState<Action>, &mut InputMap<Action>), With<InputMarker>>,
) {
    for (action_state, mut input_map) in &mut inputs {
        let toggle = action_state.just_pressed(&Action::SwitchPitchRoll);

        if toggle {
            // Me when no proper remove api
            let pitch = input_map.get(&Action::Pitch).cloned();
            let pitch_inverted = input_map.get(&Action::PitchInverted).cloned();
            let roll = input_map.get(&Action::Roll).cloned();
            let roll_inverted = input_map.get(&Action::RollInverted).cloned();

            input_map.clear_action(&Action::Pitch);
            input_map.clear_action(&Action::PitchInverted);
            input_map.clear_action(&Action::Roll);
            input_map.clear_action(&Action::RollInverted);

            if let Some(pitch) = pitch {
                for input in pitch {
                    input_map.insert(Action::Roll, input);
                }
            }

            if let Some(pitch_inverted) = pitch_inverted {
                for input in pitch_inverted {
                    input_map.insert(Action::RollInverted, input);
                }
            }

            if let Some(roll) = roll {
                for input in roll {
                    input_map.insert(Action::Pitch, input);
                }
            }

            if let Some(roll_inverted) = roll_inverted {
                for input in roll_inverted {
                    input_map.insert(Action::PitchInverted, input);
                }
            }
        }
    }
}
