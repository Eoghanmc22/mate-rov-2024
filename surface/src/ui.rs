use std::time::Duration;

use bevy::{app::AppExit, math::Vec3A, prelude::*};
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_tokio_tasks::TokioTasksRuntime;
use common::{
    components::{
        Armed, CpuTotal, CurrentDraw, Depth, DepthTarget, Inertial, LoadAverage, MeasuredVoltage,
        Memory, OrientationTarget, PwmChannel, PwmManualControl, PwmSignal, Robot, RobotId,
        RobotStatus, Temperatures,
    },
    events::ResyncCameras,
    sync::{ConnectToPeer, DisconnectPeer, Latency, MdnsPeers, Peer},
};
use egui::{load::SizedTexture, widgets, Align, Color32, Layout, RichText};
use tokio::net::lookup_host;

use crate::attitude::OrientationDisplay;

pub struct EguiUiPlugin;

impl Plugin for EguiUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin).add_systems(
            Update,
            (
                topbar,
                hud.after(topbar),
                pwm_control
                    .after(topbar)
                    .run_if(resource_exists::<PwmControl>),
                cleanup_pwm_control
                    .after(topbar)
                    .run_if(resource_removed::<PwmControl>()),
            ),
        );
    }
}

#[derive(Resource)]
pub struct ShowInspector;

#[derive(Resource)]
pub struct PwmControl(bool);

fn topbar(
    mut cmds: Commands,
    mut contexts: EguiContexts,

    robots: Query<(&Name, &RobotStatus), With<Robot>>,

    inspector: Option<Res<ShowInspector>>,
    pwm_control: Option<Res<PwmControl>>,

    peers: Query<(&Peer, Option<&Name>)>,
    mut disconnect: EventWriter<DisconnectPeer>,
) {
    egui::TopBottomPanel::top("Top Bar").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Resync Cameras").clicked() {
                    cmds.add(|world: &mut World| {
                        world.send_event(ResyncCameras);
                    })
                }

                ui.menu_button("Disconnect", |ui| {
                    if !peers.is_empty() {
                        for (peer, name) in &peers {
                            let text = if let Some(name) = name {
                                format!("{} ({})", name.as_str(), peer.token.0)
                            } else {
                                format!("{} ({})", peer.addrs, peer.token.0)
                            };

                            if ui.button(text).clicked() {
                                disconnect.send(DisconnectPeer(peer.token));
                            }
                        }
                    } else {
                        ui.label("No Connections");
                    }
                });

                if ui.button("Exit").clicked() {
                    cmds.add(|world: &mut World| {
                        world.send_event(AppExit);
                    })
                }
            });

            ui.menu_button("View", |ui| {
                if ui
                    .selectable_label(inspector.is_some(), "ECS Inspector")
                    .clicked()
                {
                    if inspector.is_some() {
                        cmds.remove_resource::<ShowInspector>()
                    } else {
                        cmds.insert_resource(ShowInspector);
                    }
                }

                if ui
                    .selectable_label(pwm_control.is_some(), "PWM Control")
                    .clicked()
                {
                    if pwm_control.is_some() {
                        cmds.remove_resource::<PwmControl>()
                    } else {
                        cmds.insert_resource(PwmControl(false));
                    }
                }
            });

            // RTL needs reverse order
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !robots.is_empty() {
                    for (robot, state) in &robots {
                        let color = match state {
                            RobotStatus::NoPeer => Color32::WHITE,
                            RobotStatus::Disarmed => Color32::RED,
                            RobotStatus::Armed => Color32::GREEN,
                        };

                        ui.label(RichText::new(format!("{state:?}")).color(color));
                        ui.label(format!("{}: ", robot.as_str()));
                    }
                } else {
                    ui.label(
                        RichText::new(format!("{:?}", RobotStatus::NoPeer)).color(Color32::WHITE),
                    );
                }
            })
        });
    });
}

fn hud(
    mut cmds: Commands,

    mut host: Local<String>,
    runtime: ResMut<TokioTasksRuntime>,

    mut contexts: EguiContexts,
    attitude: Option<Res<OrientationDisplay>>,
    robots: Query<
        (
            &Name,
            Option<&Armed>,
            Option<&MeasuredVoltage>,
            Option<&CurrentDraw>,
            Option<&CpuTotal>,
            Option<&Inertial>,
            Option<&LoadAverage>,
            Option<&Memory>,
            Option<&Temperatures>,
            Option<&Depth>,
            Option<&DepthTarget>,
            Option<&OrientationTarget>,
            Option<&Peer>,
            Option<&Latency>,
        ),
        With<Robot>,
    >,

    peers: Option<Res<MdnsPeers>>,

    mut disconnect: EventWriter<DisconnectPeer>,
) {
    let context = contexts.ctx_mut();

    // TODO(low): Support multiple robots
    if let Ok((
        robot_name,
        armed,
        voltage,
        current_draw,
        cpu,
        inertial,
        load,
        memory,
        temps,
        depth,
        depth_target,
        orientation_target,
        peer,
        latency,
    )) = robots.get_single()
    {
        let mut open = true;

        let window = egui::Window::new(robot_name.as_str())
            .id("HUD".into())
            .current_pos(context.screen_rect().right_top())
            .constrain_to(context.available_rect().shrink(20.0))
            .movable(false);

        let window = if let Some(peer) = peer {
            window.open(&mut open)
        } else {
            window
        };

        window.show(context, |ui| {
            let size = 20.0;

            if let Some(attitude) = attitude {
                let size = 250.0f32.max(ui.available_width());
                ui.image(SizedTexture::new(attitude.1, (size, size)));

                ui.add_space(10.0);
            }

            // if let Some(armed) = armed {
            //     ui.horizontal(|ui| {
            //         ui.label(RichText::new("Status:").size(size));
            //         match armed {
            //             Armed::Armed => {
            //                 ui.label(RichText::new("Armed").size(size).color(Color32::GREEN));
            //             }
            //             Armed::Disarmed => {
            //                 ui.label(RichText::new("Disarmed").size(size).color(Color32::RED));
            //             }
            //         }
            //     });
            //
            //     ui.add_space(10.0);
            // }

            if let (Some(voltage), Some(current)) = (voltage, current_draw) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Power:").size(size));

                    let voltage_color;
                    if voltage.0 .0 < 11.5 {
                        voltage_color = Color32::RED;
                    } else if voltage.0 .0 < 12.5 {
                        voltage_color = Color32::YELLOW;
                    } else {
                        voltage_color = Color32::GREEN;
                    }

                    let current_color;
                    if current.0 .0 < 15.0 {
                        current_color = Color32::GREEN;
                    } else if current.0 .0 < 20.0 {
                        current_color = Color32::YELLOW;
                    } else {
                        current_color = Color32::RED;
                    }

                    ui.label(
                        RichText::new(format!("{}", voltage.0))
                            .size(size)
                            .color(voltage_color),
                    );
                    ui.label(
                        RichText::new(format!("{}", current.0))
                            .size(size)
                            .color(current_color),
                    );
                });

                ui.add_space(10.0);
            }

            if let (Some(peer), Some(latency)) = (peer, latency) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Address:").size(size));
                    ui.label(RichText::new(format!("{:?}", peer.addrs)).size(size * 0.75));
                });

                if let Some(ping) = latency.ping {
                    ui.label(RichText::new(format!("Ping: {:.2?} frames", ping)).size(size));
                }

                ui.add_space(10.0);
            }

            if let Some(cpu) = cpu {
                ui.label(RichText::new(format!("CPU: {:.2}%", cpu.0.usage)).size(size));
            }
            if let Some(load) = load {
                ui.label(
                    RichText::new(format!(
                        "Load: {:.2}, {:.2}, {:.2}",
                        load.one_min, load.five_min, load.fifteen_min
                    ))
                    .size(size),
                );
            }

            if let Some(memory) = memory {
                let ram_usage = memory.used_mem as f64 / memory.total_mem as f64 * 100.0;
                ui.label(RichText::new(format!("RAM: {:.2}%", ram_usage)).size(size));
            }

            if cpu.is_some() || load.is_some() || memory.is_some() {
                ui.add_space(10.0);
            }

            if let Some(inertial) = inertial {
                ui.label(RichText::new(format!("IMU Temp: {}", inertial.0.tempature)).size(size));
            }

            if let Some(temps) = temps {
                for temp in &temps.0 {
                    ui.label(
                        RichText::new(format!("{}: {}", temp.name, temp.tempature)).size(size),
                    );
                }
            }

            if let Some(depth) = depth {
                ui.label(RichText::new(format!("Water Temp: {}", depth.0.temperature)).size(size));
            }

            if inertial.is_some() || temps.is_some() {
                ui.add_space(10.0);
            }

            if let Some(depth) = depth {
                ui.label(RichText::new(format!("Depth: {}", depth.0.depth)).size(size));

                if let Some(depth_target) = depth_target {
                    ui.label(RichText::new(format!("Depth Target: {}", depth_target.0)).size(size));
                }

                ui.add_space(10.0);
            }

            if let Some(orientation_target) = orientation_target {
                let target = if orientation_target.0 == Vec3A::Z {
                    "Upright"
                } else if orientation_target.0 == Vec3A::NEG_Z {
                    "Inverted"
                } else {
                    "Custom"
                };

                RichText::new(format!("Orientation Target: {target}")).size(size);
            }
        });

        if let Some(peer) = peer {
            if !open {
                disconnect.send(DisconnectPeer(peer.token));
            }
        }
    } else {
        egui::Window::new("Not Connected")
            .id("HUD".into())
            .current_pos(context.screen_rect().right_top())
            .constrain_to(context.available_rect().shrink(20.0))
            .movable(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Connect To:");
                    let line_response = ui.text_edit_singleline(&mut *host);
                    let button_response = ui.button("Connect");

                    if line_response.lost_focus() || button_response.clicked() {
                        let host = host.clone();
                        runtime.spawn_background_task(|mut ctx| async move {
                            let resolve = lookup_host(host).await;
                            let addrs = resolve.ok().and_then(|mut it| it.next());

                            if let Some(addrs) = addrs {
                                ctx.run_on_main_thread(move |ctx| {
                                    let world = ctx.world;
                                    let count = world.query::<&Robot>().iter(world).count();

                                    if count == 0 {
                                        info!("Peer ip resolved to {:?}", addrs);
                                        world.send_event(ConnectToPeer(addrs));
                                    } else {
                                        warn!("Already connected to peer");
                                    }
                                })
                                .await;
                            } else {
                                error!("Could not resolve host");
                            }
                        });
                    }
                });

                if let Some(peers) = peers {
                    let peers = &peers.0;

                    if !peers.is_empty() {
                        ui.add_space(15.0);

                        ui.heading("Peers:");

                        for peer in peers.values() {
                            let name = peer
                                .info
                                .get_fullname()
                                .split('.')
                                .next()
                                .unwrap_or("Unknown");
                            let host = peer.info.get_hostname();

                            ui.label(format!("{}@{}local", name, host));

                            ui.indent(peer.info.get_fullname(), |ui| {
                                for addrs in &peer.addresses {
                                    let addrs = *addrs;

                                    if ui.button(format!("{}", addrs.ip())).clicked() {
                                        cmds.add(move |world: &mut World| {
                                            world.send_event(ConnectToPeer(addrs));
                                        });
                                    }
                                }
                            });
                        }
                    }
                }
            });
    }
}

fn pwm_control(
    mut cmds: Commands,
    mut contexts: EguiContexts,
    mut pwm_control: ResMut<PwmControl>,
    robots: Query<(Entity, Option<&PwmManualControl>, &RobotId), With<Robot>>,
    motors: Query<(Entity, Option<&PwmSignal>, &PwmChannel, &RobotId)>,
) {
    let context = contexts.ctx_mut();
    let mut open = true;

    egui::Window::new("PWM Control")
        .current_pos(context.screen_rect().left_top())
        .constrain_to(context.available_rect().shrink(20.0))
        .open(&mut open)
        .show(contexts.ctx_mut(), |ui| {
            if let Ok((robot, manual, robot_id)) = robots.get_single() {
                let mut enabled = pwm_control.0;
                ui.checkbox(&mut enabled, "Manual Enabled");

                if enabled != pwm_control.0 || enabled != manual.is_some() {
                    pwm_control.0 = enabled;

                    if enabled {
                        info!("Enabled manual control");
                        cmds.entity(robot).insert(PwmManualControl);
                    } else {
                        info!("Disabled manual control");
                        cmds.entity(robot).remove::<PwmManualControl>();
                    }
                }

                for (motor, signal, channel, m_robot_id) in &motors {
                    if robot_id != m_robot_id {
                        continue;
                    }

                    let last_value = if let Some(signal) = signal {
                        (signal.0.as_micros() as i32 - 1500) as f32 / 400.0
                    } else {
                        0.0
                    };
                    let mut value = last_value;

                    ui.horizontal(|ui| {
                        ui.label(format!("{}", channel.0));
                        ui.add(widgets::Slider::new(&mut value, -1.0..=1.0));
                        if ui.button("Clear").clicked() {
                            value = 0.0;
                        }
                    });

                    if value != last_value {
                        let signal = 1500 + (value * 400.0) as i32;
                        cmds.entity(motor)
                            .insert(PwmSignal(Duration::from_micros(signal as u64)));
                    }
                }
            } else {
                ui.label("No robot");
            };
        });

    if !open {
        cmds.remove_resource::<PwmControl>()
    }
}

fn cleanup_pwm_control(mut cmds: Commands, robots: Query<Entity, With<Robot>>) {
    info!("Disabled manual control");
    for robot in &robots {
        cmds.entity(robot).remove::<PwmManualControl>();
    }
}
