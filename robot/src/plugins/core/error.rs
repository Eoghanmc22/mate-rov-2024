use bevy::prelude::*;
use crossbeam::channel::{self, Receiver, Sender};

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ErrorEvent>();

        let (tx, rx) = channel::bounded(30);
        app.insert_resource(Errors(tx, rx));

        app.add_systems(PostUpdate, read_errors);
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

pub fn read_errors(errors: Res<Errors>, mut events: EventReader<ErrorEvent>) {
    let error_handler = |error: &anyhow::Error| error!("Error: {error:?}");

    for error in errors.1.try_iter() {
        (error_handler)(&error)
    }

    for error in events.read() {
        (error_handler)(&error.0)
    }
}

/// For system piping
pub fn handle_errors(In(rst): In<anyhow::Result<()>>, mut events: EventWriter<ErrorEvent>) {
    if let Err(err) = rst {
        events.send(ErrorEvent(err));
    }
}
