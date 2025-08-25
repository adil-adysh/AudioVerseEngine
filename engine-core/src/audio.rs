use bevy_ecs::prelude::{Query, With, Resource, Entity};
use bevy_transform::components::Transform;
use bevy_transform::components::GlobalTransform;
use bevy_asset::Handle;
use bevy_audio::AudioSource;
use bevy_audio::PlaybackSettings;
use std::collections::HashMap;

use crate::components::*;
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;
use bevy_rapier3d::pipeline::QueryFilter;

use bevy_app::{Plugin, PostUpdate, Update};
use bevy_app::prelude::App;

// A resource to hold our audio assets.
#[derive(Resource)]
pub struct AudioAssets {
    pub sounds: HashMap<String, Handle<AudioSource>>,
}

// ... (other audio systems go here)

/// This system calculates and applies the Doppler effect based on the
/// velocities of the sound emitter and the listener.
pub fn doppler_effect_system(
    listener_query: Query<(&Transform, &Velocity), With<Player>>,
    mut emitter_query: Query<(&mut PlaybackSettings, &GlobalTransform, &Velocity)>,
) {
    if let Ok((listener_transform, listener_velocity)) = listener_query.single() {
        for (mut settings, emitter_transform, emitter_velocity) in emitter_query.iter_mut() {
            let dir = (listener_transform.translation - emitter_transform.translation()).normalize();
            let speed_of_sound: f32 = 343.0; // meters per second

            let listener_velocity_along_line = listener_velocity.0.dot(dir);
            let emitter_velocity_along_line = emitter_velocity.0.dot(dir);

            let doppler_factor: f32 = (speed_of_sound + listener_velocity_along_line)
                / (speed_of_sound + emitter_velocity_along_line);

            let new_speed: f32 = doppler_factor.clamp(0.5_f32, 2.0_f32);
            // use PlaybackSettings.speed to approximate pitch change
            settings.speed = new_speed;
        }
    }
}

// Raycast-based occlusion system (SystemParam version).
// Runs by default and uses the RapierContext SystemParam so it can be
// added directly to the Bevy schedule.
// Exclusive occlusion adapter: uses a temporary SystemState and World
// access so it can run without depending on Rapier SystemParam APIs.
// Simple occlusion system that uses material occlusion strength and
// distance to approximate obstruction without relying on Rapier. This
// allows all core systems to be enabled by default and keeps the
// implementation conservative. We can later replace this with a
// Rapier-based raycast when the environment requires it.
pub fn audio_occlusion_system(
    listener_query: Query<(&Transform,), With<Player>>,
    mut emitter_query: Query<(&mut PlaybackSettings, &GlobalTransform, Entity)>,
    material_query: Query<&SoundMaterial>,
    rapier_context: ReadRapierContext,
) {
    // Use RapierContext::cast_ray to determine if there's a blocking collider
    // between listener and emitter. We use a simple QueryFilter::default()
    // here; callers can customize colliders by adding layers or predicates in
    // the future.
    if let Ok((listener_transform,)) = listener_query.single() {
        let listener_pos = listener_transform.translation;
        for (mut settings, emitter_global, entity) in emitter_query.iter_mut() {
            let emitter_pos = emitter_global.translation();
            let dir = listener_pos - emitter_pos;
            let distance = dir.length();
            if distance <= 0.0 {
                settings.muted = false;
                continue;
            }
            let dir_norm = dir / distance;

            // Build a conservative QueryFilter. By default this will hit all
            // colliders; we could refine to ignore the emitter entity itself
            // if it has a collider.
            let filter = QueryFilter::default();

            // Acquire the RapierContext from the ReadRapierContext SystemParam
            if let Ok(rapier) = rapier_context.single() {
                // RapierContext::cast_ray expects (origin, dir, max_toi, solid, filter)
                // origin -> emitter, ray goes towards listener
                let hit = rapier.cast_ray(
                    emitter_pos.into(),
                    dir_norm.into(),
                    distance,
                    true,
                    filter,
                );

                // If a hit occurred, and material occlusion is significant, mute
                // the audio. Otherwise, do not mute.
                if let Some((_entity, _toi)) = hit {
                    if let Ok(material) = material_query.get(entity) {
                        settings.muted = material.occlusion_strength > 0.5;
                    } else {
                        settings.muted = true;
                    }
                } else {
                    settings.muted = false;
                }
            } else {
                // If we couldn't get the RapierContext, fall back to not muting.
                settings.muted = false;
            }
        }
    }
}

// Minimal audio plugin to encapsulate audio resources and systems.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioAssets {
            sounds: HashMap::new(),
        });
    app.add_systems(Update, doppler_effect_system);
    // Register occlusion system unconditionally.
    app.add_systems(PostUpdate, audio_occlusion_system);
    }
}

// Feature-gated exclusive occlusion system that used `SystemState` to
// access Rapier's `RapierContext` and regular ECS queries. An exclusive
// system was previously used to avoid `IntoSystem` conversion issues
// with the Rapier SystemParam while migrating. That adapter has been
// removed and replaced by the `audio_occlusion_system` SystemParam
// implementation above.