use std::time::Duration;

use anyhow::anyhow;
use bevy::prelude::*;

use crate::error::ErrorEvent;

pub struct OverRunPligin;

impl Plugin for OverRunPligin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverRunSettings>()
            .add_systems(Last, detect_overrun);
    }
}

#[derive(Resource)]
pub struct OverRunSettings {
    pub max_time: Duration,
}

impl Default for OverRunSettings {
    fn default() -> Self {
        Self {
            max_time: Duration::from_secs_f32(1.0 / 100.0),
        }
    }
}

fn detect_overrun(
    settings: Res<OverRunSettings>,
    time: Res<Time<Real>>,
    mut errors: EventWriter<ErrorEvent>,
) {
    let last_update = time.last_update();
    if let Some(last_update) = last_update {
        let frame_time = last_update.elapsed();

        if frame_time > settings.max_time {
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

    info!(message = "finished frame", tracy.frame_mark = true);
}
