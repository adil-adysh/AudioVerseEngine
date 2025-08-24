use bevy::prelude::{Query, Res, Entity, Transform, GlobalTransform, With, Handle, AudioSource};
use bevy_rapier3d::prelude::RapierContext;
use bevy::audio::{PlaybackSettings, Volume};
use std::collections::HashMap;

use crate::components::*;

// A resource to hold our audio assets.
#[derive(Resource)]
pub struct AudioAssets {
    pub sounds: HashMap<String, Handle<AudioSource>>,
}

// ... (other audio systems go here)

/// This system calculates and applies the Doppler effect based on the
/// velocities of the sound emitter and the listener.
pub fn doppler_effect_system(
    listener_query: Query<(&Transform, &Velocity), With<AudioListener>>,
    mut emitter_query: Query<(&mut PlaybackSettings, &GlobalTransform, &Velocity)>,
) {
    if let Ok((listener_transform, listener_velocity)) = listener_query.get_single() {
        for (mut settings, emitter_transform, emitter_velocity) in emitter_query.iter_mut() {
            let direction_to_listener = (listener_transform.translation - emitter_transform.translation()).normalize();
            let speed_of_sound = 343.0; // In meters per second.

            // Get the component of the velocities that is along the line of sight.
            let listener_velocity_along_line = listener_velocity.0.dot(direction_to_listener);
            let emitter_velocity_along_line = emitter_velocity.0.dot(direction_to_listener);
            
            // Calculate the correct Doppler factor.
            let doppler_factor = (speed_of_sound + listener_velocity_along_line) / (speed_of_sound + emitter_velocity_along_line);

            // Clamp the pitch to avoid extreme values.
            let new_pitch = doppler_factor.clamp(0.5, 2.0);
            settings.pitch = new_pitch;
        }
    }
}

/// This system uses raycasting to determine if a sound source is occluded.
/// It adjusts the sound's volume based on the `occlusion_strength` of the material.
pub fn audio_occlusion_system(
    rapier_context: Res<RapierContext>,
    listener_query: Query<(Entity, &Transform), With<AudioListener>>,
    mut emitter_query: Query<(Entity, &mut PlaybackSettings, &GlobalTransform)>,
    material_query: Query<&SoundMaterial>,
) {
    if let Ok((listener_entity, listener_transform)) = listener_query.get_single() {
        for (emitter_entity, mut settings, emitter_transform) in emitter_query.iter_mut() {
            let ray_origin = emitter_transform.translation();
            let ray_direction = listener_transform.translation - ray_origin;
            let ray_distance = ray_direction.length();
            let normalized_ray_dir = ray_direction.normalize();

            // Set up the query filter to exclude the emitter and listener.
            let filter = QueryFilter::new().exclude_entities(&[emitter_entity, listener_entity]);

            // Cast the ray.
            if let Some((entity, _intersection)) = rapier_context.cast_ray(
                ray_origin,
                normalized_ray_dir,
                ray_distance,
                true, // Solid
                filter,
            ) {
                if let Ok(material) = material_query.get(entity) {
                    settings.volume = Volume::new(1.0 - material.occlusion_strength);
                } else {
                    settings.volume = Volume::new(0.5); // Default muffling.
                }
            } else {
                // No occlusion, set volume back to normal.
                settings.volume = Volume::new(1.0);
            }
        }
    }
}