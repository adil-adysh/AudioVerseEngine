use bevy_app::Update;
use bevy_ecs::prelude::*;
use engine_core::components::{
    AudioSourceComponent as CoreAudioSourceComponent, WorldTransformComponent,
};
use engine_core::events::{
    ListenerTransformEvent, PauseSoundEvent, PlaySoundEvent, SetVolumeEvent, StopSoundEvent,
};
use engine_core::systems::ExternalAudioSystemEnabled;
use std::sync::Arc;

// Re-export Vec3 type alias from audio-system for convenience
use audio_system::AudioSystem;

#[derive(Resource, Clone)]
pub struct AudioSystemRes(pub Arc<AudioSystem>);

/// Initialize the audio-system, start a mock backend render loop, and disable core placeholder drain.
pub fn setup_audio_system(
    world: &mut World,
    sample_rate: u32,
    _channels: usize,
    frames_per_buffer: usize,
) {
    let sys = Arc::new(
        AudioSystem::new(1024, sample_rate, frames_per_buffer as u32).expect("audio-system::new"),
    );
    sys.initialize();
    // If a backend is desired, the app can start it using audio-backend and render_fn_for_system(sys.clone()).
    world.insert_resource(AudioSystemRes(sys));
    world.insert_resource(ExternalAudioSystemEnabled(true));
}

/// Drain PlaySoundEvent and start playback via audio-system helper.
pub fn play_sound_system(mut ev: ResMut<Events<PlaySoundEvent>>, sys: Res<AudioSystemRes>) {
    for e in ev.drain() {
        audio_system::AudioSystem::handle_play_sound_event(&sys.0, e);
    }
}

/// Drain StopSoundEvent and stop the entity's active sound.
pub fn stop_sound_system(mut ev: ResMut<Events<StopSoundEvent>>, sys: Res<AudioSystemRes>) {
    for e in ev.drain() {
        sys.0.stop_entity(e.entity.index());
    }
}

/// Drain PauseSoundEvent; map to volume=0.0 for now.
pub fn pause_sound_system(mut ev: ResMut<Events<PauseSoundEvent>>, sys: Res<AudioSystemRes>) {
    for e in ev.drain() {
        sys.0.set_entity_volume(e.entity.index(), 0.0);
    }
}

/// Drain SetVolumeEvent and update per-entity volume.
pub fn set_volume_system(mut ev: ResMut<Events<SetVolumeEvent>>, sys: Res<AudioSystemRes>) {
    for e in ev.drain() {
        sys.0.set_entity_volume(e.entity.index(), e.volume);
    }
}

/// Update listener position from ListenerTransformEvent.
pub fn listener_pose_system(
    mut ev: ResMut<Events<ListenerTransformEvent>>,
    sys: Res<AudioSystemRes>,
) {
    for e in ev.drain() {
        audio_system::AudioSystem::handle_listener_transform_event(&sys.0, e);
    }
}

/// Per-frame push entity world positions to the audio-system for spatialisation.
pub fn update_source_positions_system(
    sys: Res<AudioSystemRes>,
    q: Query<(Entity, &WorldTransformComponent), With<CoreAudioSourceComponent>>,
) {
    for (e, wt) in q.iter() {
        let m = wt.matrix;
        let pos = [m.w_axis.x, m.w_axis.y, m.w_axis.z];
        sys.0.set_entity_position(e.index(), pos);
    }
}

pub fn add_systems_to_engine(engine: &mut engine_core::engine::Engine) {
    engine.app_mut().add_systems(
        Update,
        (
            play_sound_system,
            stop_sound_system,
            pause_sound_system,
            set_volume_system,
            listener_pose_system,
            update_source_positions_system,
        ),
    );
}
