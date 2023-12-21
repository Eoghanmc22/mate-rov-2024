use std::{ffi::c_void, mem, sync::Arc, thread};

use anyhow::Context;
use bevy::{
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureUsages},
        texture::Volume,
    },
};
use common::{components::Camera, error::Errors};
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
                handle_added_camera,
                handle_frames.after(handle_added_camera),
            ),
        );
    }
}

#[derive(Component)]
struct VideoThread(Arc<()>, Sender<Image>, Receiver<Image>);

fn handle_added_camera(
    mut cmds: Commands,
    cameras: Query<(Entity, &Camera), Changed<Camera>>,
    mut images: ResMut<Assets<Image>>,
    errors: Res<Errors>,
) {
    for (entity, camera) in &cameras {
        cmds.entity(entity).remove::<VideoThread>();

        let handle = Arc::new(());
        let (tx_cv, rx_cv) = channel::bounded(10);
        let (tx_bevy, rx_bevy) = channel::bounded(10);

        cmds.entity(entity).insert((
            VideoThread(handle.clone(), tx_bevy, rx_cv),
            images.add(Image::default()),
        ));

        let camera = camera.clone();
        let errors = errors.0.clone();
        thread::spawn(move || {
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
            while handle.strong_count() > 0 {
                let res = src.read(&mut mat).context("Read video frame");
                let ret = match res {
                    Ok(ret) => ret,
                    Err(err) => {
                        let _ = errors.send(err);
                        continue;
                    }
                };

                images.extend(rx_bevy.try_iter());
                images.truncate(15);
                let mut image = images.pop().unwrap_or_default();

                let res = mat_to_image(&mat, &mut image).context("Mat to image");
                if let Err(err) = res {
                    let _ = errors.send(err);
                    continue;
                }

                if ret {
                    let _ = tx_cv.send(image);
                }
            }
        });
    }
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
                })
            }
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

    // TODO: Try to remove
    imgproc::cvt_color(mat, &mut out_mat, imgproc::COLOR_BGR2RGBA, 4).context("Convert colors")?;

    Ok(())
}
