use core::str;
use std::{
    io,
    net::{IpAddr, SocketAddr},
    process::{Child, Command},
    thread,
    time::Duration,
};

use ahash::{HashMap, HashSet};
use anyhow::{anyhow, bail, Context};
use bevy::{app::AppExit, prelude::*};
use common::{
    bundles::CameraBundle,
    components::{Camera, RobotId},
    ecs_sync::{NetId, Replicate},
    error::{self, Errors},
    events::ResyncCameras,
    sync::Peer,
};
use crossbeam::channel::{self, Receiver, Sender};
use tracing::{span, Level};

use crate::{
    config::RobotConfig,
    plugins::core::robot::{LocalRobot, LocalRobotMarker},
};

// TODO(low): Use multicast udp
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_camera_thread.pipe(error::handle_errors));
        app.add_systems(PreUpdate, read_new_data);
        app.add_systems(Update, handle_peers);
        app.add_systems(Last, shutdown);
    }
}

#[derive(Resource)]
struct CameraChannels(Sender<CameraEvent>, Receiver<Vec<CameraBundle>>);

enum CameraEvent {
    NewPeer(SocketAddr),
    LostPeer,
    // TODO(low): Some way to trigger this from the surface or on an interval
    Resync,
    Shutdown,
}

fn start_camera_thread(
    mut cmds: Commands,
    errors: Res<Errors>,
    robot: Res<LocalRobot>,
    config: Res<RobotConfig>,
) -> anyhow::Result<()> {
    let (tx_events, rx_events) = channel::bounded(10);
    let (tx_camreas, rx_cameras) = channel::bounded(10);

    let _ = tx_events.send(CameraEvent::Resync);

    cmds.insert_resource(CameraChannels(tx_events, rx_cameras));

    let errors = errors.0.clone();
    let robot = RobotId(robot.net_id);
    let config = config.clone();

    thread::Builder::new()
        .name("Camera Thread".to_owned())
        .spawn(move || {
            let _span = span!(Level::INFO, "Camera manager").entered();

            let mut last_cameras: HashSet<String> = HashSet::default();
            let mut cameras: HashMap<String, (Child, SocketAddr)> = HashMap::default();
            let mut target_ip = None;
            let mut port = 1024u16;

            for event in rx_events {
                match event {
                    // Respawns all instances of gstreamer and points the new ones towards the new peer
                    CameraEvent::NewPeer(addrs) => {
                        target_ip = Some(addrs.ip());

                        for (camera, (mut child, _)) in cameras.drain() {
                            let rst = child.kill();

                            if let Err(err) = rst {
                                let _ = errors.send(
                                    anyhow!(err).context(format!("Kill gstreamer for {camera}")),
                                );
                            }
                        }

                        thread::sleep(Duration::from_millis(500));

                        for camera in &last_cameras {
                            let rst = add_camera(camera, addrs.ip(), &mut cameras, &mut port);

                            if let Err(err) = rst {
                                let _ = errors.send(
                                    anyhow!(err).context(format!("Start gstreamer for {camera}")),
                                );
                            }
                        }

                        let camera_list = camera_list(&cameras, robot, &config);

                        let res = tx_camreas.send(camera_list);
                        if res.is_err() {
                            // Peer disconected
                            return;
                        }
                    }
                    CameraEvent::LostPeer => {
                        target_ip = None;

                        for (camera, (mut child, _)) in cameras.drain() {
                            let rst = child.kill();

                            if let Err(err) = rst {
                                let _ = errors.send(
                                    anyhow!(err).context(format!("Kill gstreamer for {camera}")),
                                );
                            }
                        }

                        let res = tx_camreas.send(Default::default());
                        if res.is_err() {
                            // Peer disconected
                            return;
                        }
                    }
                    // Reruns detect cameras script and start or kill instances of gstreamer as needed
                    CameraEvent::Resync => {
                        info!("Checking for new cameras");

                        let camera_detect =
                            Command::new("/home/pi/mate/detect_cameras.sh").output();

                        match camera_detect {
                            Ok(output) => {
                                if !output.status.success() {
                                    let _ =
                                        errors.send(anyhow!("Collect cameras: {}", output.status));
                                    continue;
                                }

                                match str::from_utf8(&output.stdout) {
                                    Ok(data) => {
                                        let next_cameras: HashSet<String> =
                                            data.lines().map(ToOwned::to_owned).collect();

                                        for old_camera in last_cameras.difference(&next_cameras) {
                                            if let Some(mut child) = cameras.remove(old_camera) {
                                                let rst = child.0.kill();

                                                if let Err(err) = rst {
                                                    let _ = errors.send(anyhow!(err).context(
                                                        format!("Kill gstreamer for {old_camera}"),
                                                    ));
                                                }
                                            } else {
                                                error!("Attempted to remove a nonexistant camera");
                                            }
                                        }

                                        for new_camera in next_cameras.difference(&last_cameras) {
                                            if let Some(ip) = target_ip {
                                                let rst = add_camera(
                                                    new_camera,
                                                    ip,
                                                    &mut cameras,
                                                    &mut port,
                                                );

                                                if let Err(err) = rst {
                                                    let _ = errors.send(anyhow!(err).context(
                                                        format!("Start gstreamer for {new_camera}"),
                                                    ));
                                                }
                                            } else {
                                                error!("Tried to update cameras without a peer");
                                            }
                                        }

                                        last_cameras = next_cameras;

                                        let camera_list = camera_list(&cameras, robot, &config);
                                        let res = tx_camreas.send(camera_list);
                                        if res.is_err() {
                                            // Peer disconected
                                            return;
                                        }
                                    }
                                    Err(err) => {
                                        let _ =
                                            errors.send(anyhow!(err).context("Collect cameras"));
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = errors.send(anyhow!(err).context("Collect cameras"));
                            }
                        }
                    }
                    CameraEvent::Shutdown => {
                        for (camera, (mut child, _)) in cameras.drain() {
                            let rst = child.kill();

                            if let Err(err) = rst {
                                let _ = errors.send(
                                    anyhow!(err).context(format!("Kill gstreamer for {camera}")),
                                );
                            }
                        }

                        let res = tx_camreas.send(Default::default());
                        if res.is_err() {
                            // Peer disconected
                            return;
                        }

                        return;
                    }
                }
            }
        })
        .context("Spawn thread")?;

    Ok(())
}

fn handle_peers(
    channels: Res<CameraChannels>,
    mut disconnected: RemovedComponents<Peer>,
    connected: Query<&Peer, Changed<Peer>>,
    mut resync_events: EventReader<ResyncCameras>,
) {
    let mut event = None;

    for _resync in resync_events.read() {
        event = Some(CameraEvent::Resync);
    }

    for _disconnection in disconnected.read() {
        event = Some(CameraEvent::LostPeer);
    }

    for peer in connected.iter() {
        event = Some(CameraEvent::NewPeer(peer.addrs));
    }

    if let Some(event) = event {
        let res = channels.0.send(event);
        if let Err(_) = res {
            error!("Camera thread dead");
        }
    }
}

// TODO(low): Only update the cameras that changed
fn read_new_data(
    mut cmds: Commands,
    channels: Res<CameraChannels>,
    robot: Query<(Entity, &NetId), With<LocalRobotMarker>>,
    cameras: Query<(Entity, &RobotId), With<Camera>>,
) {
    let mut new_cameras = None;
    for camera_update in channels.1.try_iter() {
        new_cameras = Some(camera_update);
    }

    if let Some(new_cameras) = new_cameras {
        let (_robot, id) = robot.single();

        for (entity, camera_robot) in &cameras {
            if camera_robot.0 == *id {
                cmds.entity(entity).despawn();
            }
        }

        for camera in new_cameras {
            cmds.spawn((camera, Replicate));
        }
    }
}

fn shutdown(channels: Res<CameraChannels>, mut exit: EventReader<AppExit>) {
    for _event in exit.read() {
        let _ = channels.0.send(CameraEvent::Shutdown);
    }
}

/// Spawns a gstreamer with the args necessary
fn start_gstreamer(camera: &str, addrs: SocketAddr) -> io::Result<Child> {
    Command::new("gst-launch-1.0")
        .arg("v4l2src")
        .arg(format!("device={camera}"))
        .arg("!")
        .arg("video/x-h264,width=1920,height=1080,framerate=30/1")
        .arg("!")
        .arg("rtph264pay")
        .arg("!")
        .arg("udpsink")
        .arg(format!("host={}", addrs.ip()))
        .arg(format!("port={}", addrs.port()))
        .spawn()
}

/// Starts a gstreamer and updates state
fn add_camera(
    camera: &str,
    ip: IpAddr,
    cameras: &mut HashMap<String, (Child, SocketAddr)>,
    port: &mut u16,
) -> anyhow::Result<()> {
    let setup_exit = Command::new("/home/pi/mate/setup_camera.sh")
        .arg(camera)
        .spawn()
        .context("Setup cameras")?
        .wait()
        .context("wait on setup")?;
    if !setup_exit.success() {
        bail!("Could not setup cameras");
    }

    let bind = (ip, *port).into();
    let child =
        start_gstreamer(camera, bind).with_context(|| format!("Spawn gstreamer for {camera}"))?;
    *port += 1;

    cameras.insert((*camera).to_owned(), (child, bind));

    Ok(())
}

/// Converts internal repersentation of cameras to what the protocol calls for
fn camera_list(
    cameras: &HashMap<String, (Child, SocketAddr)>,
    robot: RobotId,
    config: &RobotConfig,
) -> Vec<CameraBundle> {
    let mut list = Vec::new();

    for (name, &(_, location)) in cameras {
        let (name, transform) = match config.cameras.get(name) {
            Some(definition) => (
                format!("{} ({})", definition.name, name),
                definition.transform.flatten(),
            ),
            None => (name.to_owned(), Transform::default()),
        };

        list.push(CameraBundle {
            name: Name::new(name),
            camera: Camera { location },
            robot,
            transform,
        });
    }

    list
}
