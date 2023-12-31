use std::time::{Duration, Instant};

use anyhow::anyhow;
use bevy::prelude::*;

use crate::error::ErrorEvent;

pub struct OverRunPligin;

impl Plugin for OverRunPligin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverRunSettings>()
            .add_systems(First, begin_tick)
            // TODO(low): run before error system
            .add_systems(Last, detect_overrun);
    }
}

#[derive(Resource)]
pub struct OverRunSettings {
    pub max_time: Duration,
    pub tracy_frame_mark: bool,
}

impl Default for OverRunSettings {
    fn default() -> Self {
        Self {
            max_time: Duration::from_secs_f32(1.0 / 100.0),
            tracy_frame_mark: true,
        }
    }
}

#[derive(Resource)]
pub struct TickStart(Instant);

fn begin_tick(mut cmds: Commands) {
    cmds.insert_resource(TickStart(Instant::now()))
}

const TOLERANCE: Duration = Duration::from_micros(300);

fn detect_overrun(
    settings: Res<OverRunSettings>,
    start: Option<Res<TickStart>>,
    mut errors: EventWriter<ErrorEvent>,
) {
    if let Some(start) = start {
        let frame_time = start.0.elapsed();

        if frame_time > settings.max_time + TOLERANCE {
            errors.send(
                anyhow!(
                    "Max loop time over run. Last tick took {:.4}, exceeding limit of {:.4}",
                    frame_time.as_secs_f32(),
                    settings.max_time.as_secs_f32()
                )
                .into(),
            )
        }
    }

    #[cfg(feature = "tracy_frame_mark")]
    if settings.tracy_frame_mark {
        info!(message = "finished frame", tracy.frame_mark = true);
    }
}
