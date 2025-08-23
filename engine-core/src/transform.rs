use bevy_ecs::prelude::*;

use crate::components::{TransformComponent, WorldTransformComponent};

/// Computes world-space transform matrices for entities.
/// This variant targets the physics step (no interpolation).
pub fn physics_transform_system(
    query: Query<(Entity, &TransformComponent, Option<&WorldTransformComponent>)>,
    parents: Query<&WorldTransformComponent>,
    mut commands: Commands,
) {
    for (entity, local, _world_opt) in query.iter() {
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
) {
    // For now identical to physics; can diverge later (interpolation, smoothing).
    physics_transform_system(query, parents, commands);
}
