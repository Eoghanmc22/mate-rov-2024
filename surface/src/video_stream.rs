use std::{borrow::Cow, ffi::c_void, mem, sync::Arc, thread};

use anyhow::{anyhow, Context};
use bevy::{
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureUsages},
        texture::Volume,
    },
};
use common::{
    components::Camera,
    error::{self, ErrorEvent, Errors},
};
use crossbeam::channel::{self, Receiver, Sender};
use opencv::{
    imgproc,
    platform_types::size_t,
    prelude::*,
    videoio::{self, VideoCapture},
};

pub struct VideoStreamPlugin;

impl Plugin for VideoStreamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_added_camera
                    .pipe(error::handle_errors)
                    .before(handle_frames),
                handle_frames,
                handle_video_processors,
            ),
        );
    }
}

/// An interface to plug into the video streaming pipeline
pub trait VideoProcessor: Send + 'static {
    fn begin(&mut self);
    fn process<'b, 'a: 'b>(&'a mut self, img: &'b mut Mat) -> anyhow::Result<&'b Mat>;
    fn should_end(&self) -> bool {
        false
    }
    fn end(&mut self);
}
type BoxedVideoProcessor = Box<dyn VideoProcessor>;

#[derive(Component, Clone)]
pub struct VideoProcessorFactory(
    pub Cow<'static, str>,
    pub fn(&mut World) -> BoxedVideoProcessor,
);

#[derive(Component)]
pub struct VideoThread(
    // Used by the video thread to detect when its handle is droped from the ECS
    Arc<()>,
    // Channels for displaying and reusing bevy images
    Sender<Image>,
    Receiver<Image>,
    // Channel to update the thread's VideoProcessor
    Sender<Option<BoxedVideoProcessor>>,
);

fn handle_added_camera(
    mut cmds: Commands,
    cameras: Query<(Entity, &Camera), Changed<Camera>>,
    mut images: ResMut<Assets<Image>>,
    errors: Res<Errors>,
) -> anyhow::Result<()> {
    for (entity, camera) in &cameras {
        cmds.entity(entity).remove::<VideoThread>();

        let handle = Arc::new(());
        let (tx_cv, rx_cv) = channel::bounded(10);
        let (tx_bevy, rx_bevy) = channel::bounded(10);
        let (tx_proc, rx_proc) = channel::bounded(10);

        cmds.entity(entity).insert((
            VideoThread(handle.clone(), tx_bevy, rx_cv, tx_proc),
            images.add(Image::default()),
        ));

        let camera = camera.clone();
        let errors = errors.0.clone();
        thread::Builder::new()
            .name("Video Thread".to_owned())
            .spawn(move || {
                let handle = Arc::downgrade(&handle);
                let mut images: Vec<Image> = Vec::new();

                let src = VideoCapture::from_file(&gen_src(&camera), videoio::CAP_GSTREAMER);
                let mut src = match src.context("Open video capture") {
                    Ok(src) => src,
                    Err(err) => {
                        let _ = errors.send(err);
                        return;
                    }
                };

                // Loop until the VideoThread component is dropped
                let mut mat = Mat::default();
                let mut proc: Option<BoxedVideoProcessor> = None;

                while handle.strong_count() > 0 {
                    let res = src.read(&mut mat).context("Read video frame");

                    let new_frame = match res {
                        Ok(ret) => ret,
                        Err(err) => {
                            let _ = errors.send(err);
                            continue;
                        }
                    };

                    if let Some(mut new_proc) = rx_proc.try_iter().last() {
                        if let Some(proc) = &mut proc {
                            proc.end();
                        }

                        if let Some(new_proc) = &mut new_proc {
                            new_proc.begin();
                        }

                        proc = new_proc;
                    }

                    if new_frame {
                        let mat = if let Some(proc_local) = &mut proc {
                            if !proc_local.should_end() {
                                let res = proc_local.process(&mut mat);

                                match res {
                                    Ok(mat) => mat,
                                    Err(err) => {
                                        let _ = errors.send(err);
                                        &mat
                                    }
                                }
                            } else {
                                proc_local.end();
                                proc = None;

                                &mat
                            }
                        } else {
                            &mat
                        };

                        images.extend(rx_bevy.try_iter());
                        images.truncate(15);
                        let mut image = images.pop().unwrap_or_default();

                        let res = mat_to_image(mat, &mut image).context("Mat to image");
                        if let Err(err) = res {
                            let _ = errors.send(err);
                            continue;
                        }

                        let _ = tx_cv.send(image);
                    }
                }

                if let Some(proc) = &mut proc {
                    proc.end();
                }
            })
            .context("Spawn thread")?;
    }

    Ok(())
}

fn handle_frames(
    cameras: Query<
        (
            &VideoThread,
            &Handle<Image>,
            Option<&Handle<StandardMaterial>>,
        ),
        With<Camera>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut image_events: EventWriter<AssetEvent<StandardMaterial>>,
) {
    for (thread, handle, material) in &cameras {
        let latest = thread.2.try_iter().fold(None, |last, next| {
            if let Some(last) = last {
                let _ = thread.1.send(last);
            }

            Some(next)
        });

        if let Some(latest) = latest {
            let Some(image) = images.get_mut(handle) else {
                warn!("Couldnt get render asset for image");
                continue;
            };
            let old = mem::replace(image, latest);
            let _ = thread.1.send(old);

            // This shouldnt be the responsibility of this system but oh well
            if let Some(material) = material {
                image_events.send(AssetEvent::Modified {
                    id: material.into(),
                });
            }
        }
    }
}

fn handle_video_processors(
    mut cmds: Commands,

    cameras: Query<&VideoThread, With<Camera>>,
    cameras_with_processor: Query<(&VideoThread, Ref<VideoProcessorFactory>), With<Camera>>,
    mut removed: RemovedComponents<VideoProcessorFactory>,
    mut errors: EventWriter<ErrorEvent>,
) {
    for entity in removed.read() {
        if let Ok(thread) = cameras.get(entity) {
            let rst = thread.3.send(None);
            if rst.is_err() {
                errors.send(anyhow!("Could not remove video processor").into());
            }
        } else {
            // The whole entity probably despawned and the video thread will shutdown
        }
    }

    for (thread, processor) in &cameras_with_processor {
        if processor.is_changed() {
            let proc_tx = thread.3.clone();
            let processor = processor.1;

            cmds.add(move |world: &mut World| {
                let processor = (processor)(world);

                let rst = proc_tx.send(Some(processor));
                if rst.is_err() {
                    let _ = world
                        .resource::<Errors>()
                        .0
                        .send(anyhow!("Could not send new video processor"));
                }
            });
        }
    }
}

/// Generates the gstreamer pipeline to recieve data from `camera`
fn gen_src(camera: &Camera) -> String {
    let ip = camera.location.ip();
    let port = camera.location.port();

    format!("udpsrc address={ip} port={port} caps=application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,a-framerate=30,payload=96 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! video/x-raw,format=BGR ! appsink drop=1")
}

/// Efficiently converts opencv `Mat`s to bevy `Image`s
fn mat_to_image(mat: &Mat, image: &mut Image) -> anyhow::Result<()> {
    // Convert opencv size to bevy size
    let size = mat.size().context("Get size")?;
    let extent = Extent3d {
        width: size.width as u32,
        height: size.height as u32,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.size = extent;
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    // Allocate bevy image if needed
    let cap = extent.volume() * 4;
    image.data.clear();
    image.data.reserve(cap);

    // Make the bevy image into a opencv mat
    // SAFETY: The vector outlives the returned mat and we dont do anything that could cause the
    // vec to re allocate until after the mat gets dropped
    let mut out_mat = unsafe {
        let dst_ptr = image.data.as_mut_ptr() as *mut c_void;
        let dst_step = size.width as size_t * 4;

        let out_mat = Mat::new_rows_cols_with_data(
            size.height,
            size.width,
            opencv::core::CV_8UC4,
            dst_ptr,
            dst_step,
        )
        .context("Convert colors")?;
        image.data.set_len(cap);

        out_mat
    };

    // TODO(mid): Try to remove
    imgproc::cvt_color(mat, &mut out_mat, imgproc::COLOR_BGR2RGBA, 4).context("Convert colors")?;

    Ok(())
}
