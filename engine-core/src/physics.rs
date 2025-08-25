use bevy_ecs::prelude::{Query, Res, With};
use bevy_time::Time;
use bevy_transform::components::Transform;
use bevy_math::Vec3;
use bevy_rapier3d::prelude::{KinematicCharacterController, KinematicCharacterControllerOutput};

use crate::components::*;

/// This system handles all kinematic player movement logic.
/// It combines input and external forces to set the desired translation.
/// It relies on Rapier's internal logic to handle slopes and collisions.
pub fn kinematic_controller_update_system(
    mut player_query: Query<(
        &MovementSpeed,
        &mut KinematicCharacterController,
        &MoveDirection,
        Option<&ExternalForce>,
    ), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((speed, mut controller, direction, external_force)) = player_query.single_mut() {
    let delta_time = time.delta_secs();
        let mut desired_translation = Vec3::ZERO;

        // 1. Add player's intended movement.
        desired_translation += direction.0 * speed.0 * delta_time;

        // 2. Add any external forces.
        if let Some(force) = external_force {
            desired_translation += force.0 * delta_time;
        }
        
        // 3. Update the controller's translation with the final, combined movement.
        // Rapier will handle the collision resolution, including slopes and steps.
    controller.translation = Some(desired_translation);
    }
}

/// This system runs AFTER the physics step. It reads the final physics
/// output to update the player's true velocity and orientation.
pub fn update_player_state_system(
    mut player_query: Query<(
        &KinematicCharacterControllerOutput,
        &mut Velocity,
        &mut Transform,
    ), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((output, mut velocity, mut transform)) = player_query.single_mut() {
        // 1. Update the velocity with the effective translation from Rapier.
    let dt = time.delta_secs();
        if dt > 0.0 {
            velocity.0 = output.effective_translation / dt;
        }

        // 2. Update the player's facing direction based on their true velocity.
        if velocity.0.length_squared() > 1e-6 {
            // Use Transform::look_at with bevy_math Vec3 types.
            let target = transform.translation + velocity.0.normalize();
            transform.look_at(target, Vec3::Y);
        }
    }
}