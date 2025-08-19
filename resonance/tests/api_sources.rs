use resonance::*;

#[test]
fn test_create_and_destroy_sources() {
    let api = ResonanceAudioApi::new(2, 256, 48000).expect("Failed to create ResonanceAudioApi");

    // Create different types of sources via the safe wrapper and then destroy them.
    let amb_id = api.create_ambisonic_source(4);
    let stereo_id = api.create_stereo_source(2);
    // Use rendering mode 0 as a safe default for tests.
    let sound_id = api.create_sound_object_source(0);

    // IDs should be non-negative (C API returns int)
    assert!(amb_id >= 0, "ambisonic id must be >= 0");
    assert!(stereo_id >= 0, "stereo id must be >= 0");
    assert!(sound_id >= 0, "sound object id must be >= 0");

    // Set some source properties (no panic) via the wrapper
    api.set_source_position(amb_id, 1.0, 0.0, 0.0);
    api.set_source_volume(stereo_id, 0.75);
    api.set_source_rotation(sound_id, 0.0, 0.0, 0.0, 1.0);
    api.set_source_room_effects_gain(amb_id, 0.5);

    // Destroy sources via the wrapper
    api.destroy_source(amb_id);
    api.destroy_source(stereo_id);
    api.destroy_source(sound_id);

    drop(api);
}
