use bevy_ecs::{
    event::EventReader,
    system::{Commands, Res, ResMut, SystemChangeTick},
    world::World,
};
use tracing::error;

use super::{
    EntityMap, Replicate, SerializationSettings, SerializedChange, SerializedChangeInEvent,
};

pub fn apply_changes(
    mut cmds: Commands,

    ticks: SystemChangeTick,
    settings: Res<SerializationSettings>,
    mut entity_map: ResMut<EntityMap>,
    mut reader: EventReader<SerializedChangeInEvent>,
) {
    for SerializedChangeInEvent(change) in reader.read() {
        match change {
            SerializedChange::EntitySpawned(forign) => {
                let local = cmds.spawn((Replicate, *forign)).id();

                entity_map.local_to_forign.insert(local, *forign);
                entity_map.forign_to_local.insert(*forign, local);

                entity_map.local_modified.insert(local, ticks.this_run());
            }
            SerializedChange::EntityDespawned(forign) => {
                let Some(local) = entity_map.forign_to_local.remove(forign) else {
                    error!("Got despawn for unknown entity");
                    continue;
                };
                entity_map.local_to_forign.remove(&local);
                entity_map.local_modified.remove(&local);

                cmds.entity(local).despawn();
            }
            SerializedChange::ComponentUpdated(forign, token, Some(serialized)) => {
                let Some(&local) = entity_map.forign_to_local.get(forign) else {
                    error!("Got update for unknown entity");
                    continue;
                };

                let Some(&component_id) = settings.component_lookup.get(token) else {
                    error!("Got update for unknown entity token");
                    continue;
                };

                let Some(sync_info) = settings.tracked_components.get(&component_id) else {
                    unreachable!();
                };

                let type_adapter = sync_info.adapter.clone();
                let serialized = serialized.clone();

                cmds.add(move |world: &mut World| {
                    // TODO: error handling
                    type_adapter
                        .deserialize(&serialized, &mut |ptr|
                        // SAFETY: We used the type adapter associated with this component id
                        unsafe {
                            world.entity_mut(local).insert_by_id(component_id, ptr);
                        })
                        .expect("Bad update");
                });

                entity_map.local_modified.insert(local, ticks.this_run());
            }
            SerializedChange::ComponentUpdated(forign, token, None) => {
                let Some(&local) = entity_map.forign_to_local.get(forign) else {
                    error!("Got update for unknown entity");
                    continue;
                };

                let Some(&component_id) = settings.component_lookup.get(token) else {
                    error!("Got update for unknown entity token");
                    continue;
                };

                let Some(sync_info) = settings.tracked_components.get(&component_id) else {
                    unreachable!();
                };

                // TODO: there doesnt seem to be a bevy api for this...
                let remover = sync_info.remove_fn;
                cmds.add(move |world: &mut World| {
                    let mut entity = world.entity_mut(local);
                    (remover)(&mut entity);
                });

                entity_map.local_modified.insert(local, ticks.this_run());
            }
            SerializedChange::EventEmitted(_, _) => todo!(),
        }
    }
}
