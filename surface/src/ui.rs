use bevy::{math::Vec3A, prelude::*};
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_tokio_tasks::TokioTasksRuntime;
use common::{
    components::{
        Armed, CpuTotal, Depth, DepthTarget, Inertial, LoadAverage, Memory, OrientationTarget,
        Robot, Temperatures,
    },
    sync::ConnectToPeer,
};
use egui::{load::SizedTexture, Color32, RichText};
use tokio::net::lookup_host;

use crate::attitude::OrientationDisplay;

pub struct EguiUiPlugin;

impl Plugin for EguiUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_systems(Update, (topbar, hud.after(topbar)));
    }
}

fn topbar(mut contexts: EguiContexts) {
    egui::TopBottomPanel::top("Top Bar").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| ui.menu_button("test", |ui| ui.button("thing")))
    });
}

fn hud(
    mut host: Local<String>,
    runtime: ResMut<TokioTasksRuntime>,

    mut contexts: EguiContexts,
    attitude: Option<Res<OrientationDisplay>>,
    robots: Query<
        (
            &Name,
            Option<&Armed>,
            Option<&CpuTotal>,
            Option<&Inertial>,
            Option<&LoadAverage>,
            Option<&Memory>,
            Option<&Temperatures>,
            Option<&Depth>,
            Option<&DepthTarget>,
            Option<&OrientationTarget>,
        ),
        With<Robot>,
    >,
) {
    let context = contexts.ctx_mut();

    // TODO: Support multiple robots
    if let Ok((
        robot_name,
        armed,
        cpu,
        inertial,
        load,
        memory,
        temps,
        depth,
        depth_target,
        orientation_target,
    )) = robots.get_single()
    {
        egui::Window::new(robot_name.as_str())
            .id("HUD".into())
            .current_pos(context.screen_rect().right_top())
            .constraint_to(context.available_rect().shrink(20.0))
            .movable(false)
            .show(context, |ui| {
                // TODO: Ping

                // Armed
                // CPU utilization
                // Temps
                // Attitude
                // Amperage/power data?

                let size = 20.0;

                if let Some(attitude) = attitude {
                    ui.image(SizedTexture::new(attitude.1, (250.0, 250.0)));

                    ui.add_space(10.0);
                }

                if let Some(armed) = armed {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Status:").size(size));
                        match armed {
                            Armed::Armed => {
                                ui.label(RichText::new("Armed").size(size).color(Color32::GREEN));
                            }
                            Armed::Disarmed => {
                                ui.label(RichText::new("Disarmed").size(size).color(Color32::RED));
                            }
                        }
                    });

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
                    ui.label(
                        RichText::new(format!("IMU Temp: {}", inertial.0.tempature)).size(size),
                    );
                }

                if let Some(temps) = temps {
                    for temp in &temps.0 {
                        ui.label(
                            RichText::new(format!("{}: {}", temp.name, temp.tempature)).size(size),
                        );
                    }
                }

                if let Some(depth) = depth {
                    ui.label(
                        RichText::new(format!("Water Temp: {}", depth.0.temperature)).size(size),
                    );
                }

                if inertial.is_some() || temps.is_some() {
                    ui.add_space(10.0);
                }

                if let Some(depth) = depth {
                    ui.label(RichText::new(format!("Depth: {}", depth.0.depth)).size(size));

                    if let Some(depth_target) = depth_target {
                        ui.label(
                            RichText::new(format!("Depth Target: {}", depth_target.0)).size(size),
                        );
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
    } else {
        egui::Window::new("Not Connected")
            .id("HUD".into())
            .current_pos(context.screen_rect().right_top())
            .constraint_to(context.available_rect().shrink(20.0))
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
            });
    }
}
