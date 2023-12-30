use bevy::prelude::*;
use crossbeam::channel::{self, Receiver, Sender};

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ErrorEvent>();

        let (tx, rx) = channel::bounded(30);
        app.insert_resource(Errors(tx, rx));

        app.add_systems(Last, (error_channel, read_errors.after(error_channel)));
    }
}

#[derive(Resource)]
pub struct Errors(pub Sender<anyhow::Error>, Receiver<anyhow::Error>);

#[derive(Event)]
pub struct ErrorEvent(pub anyhow::Error);

impl From<anyhow::Error> for ErrorEvent {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

pub fn error_channel(errors: Res<Errors>, mut events: EventWriter<ErrorEvent>) {
    for error in errors.1.try_iter() {
        events.send(ErrorEvent(error));
    }
}

pub fn read_errors(mut events: EventReader<ErrorEvent>) {
    for ErrorEvent(error) in events.read() {
        error!("Error: {error:?}");
    }
}

/// For system piping
pub fn handle_errors(In(rst): In<anyhow::Result<()>>, mut events: EventWriter<ErrorEvent>) {
    if let Err(err) = rst {
        events.send(ErrorEvent(err));
    }
}
