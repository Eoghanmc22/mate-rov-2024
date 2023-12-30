use tracy_client::{Frame, FrameName};

pub fn secondary_frame_mark(name: FrameName) {
    let client = tracy_client::Client::running();
    if let Some(client) = client {
        client.secondary_frame_mark(name)
    }
}

pub fn non_continuous_frame(name: FrameName) -> Option<Frame> {
    let client = tracy_client::Client::running();
    client.map(|client| client.non_continuous_frame(name))
}
