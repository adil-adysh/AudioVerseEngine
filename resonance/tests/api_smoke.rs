use resonance::*;

#[test]
fn test_api_create_and_destroy() {
    // Try to create the API with typical parameters
    let api = ResonanceAudioApi::new(2, 256, 48000).expect("Failed to create ResonanceAudioApi");
    // Explicitly drop to ensure native resource is released now.
    drop(api);
}

#[test]
fn test_api_methods_do_not_panic() {
    let api = ResonanceAudioApi::new(2, 256, 48000).expect("Failed to create ResonanceAudioApi");
    api.set_head_position(0.0, 0.0, 0.0);
    api.set_head_rotation(0.0, 0.0, 0.0, 1.0);
    api.set_master_volume(1.0);
    api.set_stereo_speaker_mode(false);
    api.enable_room_effects(true);
    drop(api);
}
