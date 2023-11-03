use anyhow::Context;
use bevy::{app::AppExit, prelude::*};
use crossbeam::channel::{self, Receiver};

use super::error;

pub struct CtrlCPlugin;

impl Plugin for CtrlCPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_handler.pipe(error::handle_errors));
        app.add_systems(PreUpdate, check_handler);
    }
}

#[derive(Resource)]
struct CtrlcChannel(Receiver<()>);

pub fn setup_handler(mut cmds: Commands) -> anyhow::Result<()> {
    let (tx, rx) = channel::bounded(1);

    cmds.insert_resource(CtrlcChannel(rx));

    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .context("Set ctrl-c")?;

    Ok(())
}

pub fn check_handler(channel: Res<CtrlcChannel>, mut exit: EventWriter<AppExit>) {
    if let Ok(()) = channel.0.try_recv() {
        exit.send(AppExit);
    }
}
