use bevy_ecs::{event::EventReader, system::ResMut};

use super::{SerializedChangeEventIn, SyncState};

pub fn detect_changes(
    // mut cmds: Commands,
    // world: &World,
    // tick: SystemChangeTick,
    //
    // mut state: Local<ChangeDetectionState>,
    // settings: Res<ChangeDetectionSettings>,
    mut sync_state: ResMut<SyncState>,

    mut changes: EventReader<SerializedChangeEventIn>,
) {
    for ch
}
