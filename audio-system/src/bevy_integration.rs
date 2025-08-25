#[cfg(feature = "bevy-integration")]
use bevy::prelude::*;
#[cfg(feature = "bevy-integration")]
use bevy_oddio::frames::Stereo;
#[cfg(feature = "bevy-integration")]
use bevy_oddio::Audio as BevyAudio;

// Helper systems and wrappers to connect engine events to bevy_oddio playback.
#[cfg(feature = "bevy-integration")]
pub fn play_sound_event_system(mut commands: Commands, audio: ResMut<BevyAudio<Stereo>>, asset_server: Res<AssetServer>, mut query: Query<&engine_core::audio::PlaySoundEvent>) {
    // This is a placeholder: real implementation should read events from Resources or EventReader
    for ev in query.iter_mut() {
        // For now, play a default sine or an asset path if provided
        let handle = if ev.asset_path.is_empty() {
            // builtin sine via bevy_oddio builtins
            // audio.play(bevy_oddio::builtins::sine::Sine::default(), 0.0);
            continue;
        } else {
            let h = asset_server.load(&ev.asset_path);
            audio.play_spatial(h, Default::default(), oddio::spatial::SpatialOptions::default());
            continue;
        };
    }
}
