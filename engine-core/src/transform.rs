use bevy_ecs::prelude::*;

use crate::components::{AudioSourceComponent, TransformComponent, WorldTransformComponent};
use crate::events::PositionChangedEvent;

/// Computes world-space transform matrices for entities.
/// This variant targets the physics step (no interpolation).
pub fn physics_transform_system(
    query: Query<(Entity, &TransformComponent, Option<&WorldTransformComponent>)>,
    parents: Query<&WorldTransformComponent>,
    mut commands: Commands,
    mut pos_events: ResMut<bevy_ecs::event::Events<PositionChangedEvent>>,
) {
    for (entity, local, world_opt) in query.iter() {
        let parent_mat = match local.parent {
            Some(p) => parents.get(p).map(|w| w.matrix).unwrap_or(glam::Mat4::IDENTITY),
            None => glam::Mat4::IDENTITY,
        };

        let local_mat = glam::Mat4::from_scale_rotation_translation(
            local.scale,
            local.rotation,
            local.position,
        );
        let world_mat = parent_mat * local_mat;

        // Emit position-changed if position changed
        if let Some(prev) = world_opt {
            let prev_pos = prev.matrix.w_axis.truncate();
            let new_pos = world_mat.w_axis.truncate();
            if (prev_pos - new_pos).length_squared() > 0.0 {
                pos_events.send(PositionChangedEvent { entity, position: new_pos });
            }
        } else {
            // first-time insertion counts as change
            pos_events.send(PositionChangedEvent { entity, position: world_mat.w_axis.truncate() });
        }
        // Insert will overwrite if it exists already.
        commands
            .entity(entity)
            .insert(WorldTransformComponent { matrix: world_mat });
    }
}

/// Computes world-space transform matrices for rendering (future: add interpolation).
pub fn render_transform_system(
    query: Query<(Entity, &TransformComponent, Option<&WorldTransformComponent>)>,
    parents: Query<&WorldTransformComponent>,
    commands: Commands,
    pos_events: ResMut<bevy_ecs::event::Events<PositionChangedEvent>>,
) {
    // For now identical to physics; can diverge later (interpolation, smoothing).
    physics_transform_system(query, parents, commands, pos_events);
}

/// Cleanup hook: when an AudioSourceComponent is removed or an entity despawns, emit StopSoundEvent.
pub fn despawn_cleanup_system(
    mut removed_sources: RemovedComponents<AudioSourceComponent>,
    mut stop_events: ResMut<bevy_ecs::event::Events<crate::events::StopSoundEvent>>,
) {
    for e in removed_sources.iter() {
        stop_events.send(crate::events::StopSoundEvent { entity: e });
    }
}
