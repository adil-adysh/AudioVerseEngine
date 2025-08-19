use resonance::*;
use std::mem;

#[test]
#[ignore]
fn test_planar_buffers_and_distance_model() {
    // Requires native resonance-audio build; enabled by setting RUN_DEEP_INTEGRATION=1
    if std::env::var("RUN_DEEP_INTEGRATION").is_err() {
        eprintln!("skipping deep integration test: set RUN_DEEP_INTEGRATION=1 to run");
        return;
    }
    let api = ResonanceAudioApi::new(4, 256, 48000).expect("Failed to create API");

    // Create an ambisonic source (planar buffer usage)
    let amb_id = api.create_ambisonic_source(4);
    assert!(amb_id >= 0);

    // Prepare planar buffers (array of pointers) - simulate 4 channels with small frames
    let frames = 8usize;
    // We can't call planar setter wrapper (not implemented), but call interleaved
    // as an alternative for this test to ensure buffer-setting APIs work.
    let interleaved = vec![0.0f32; 4 * frames];
    eprintln!("[deep-test] calling set_interleaved_buffer_f32");
    api.set_interleaved_buffer_f32(amb_id, &interleaved, 4, frames);
    eprintln!("[deep-test] returned from set_interleaved_buffer_f32");

    // Test distance model setter and attenuation
    eprintln!("[deep-test] calling set_source_distance_attenuation");
    api.set_source_distance_attenuation(amb_id, 0.8);
    eprintln!("[deep-test] calling set_source_distance_model");
    // Use rolloff 0 as a safe enum value for tests
    api.set_source_distance_model(amb_id, 0, 0.5, 100.0);
    eprintln!("[deep-test] returned from distance model calls");

    eprintln!("[deep-test] destroying source");
    api.destroy_source(amb_id);
    eprintln!("[deep-test] destroyed source");
    eprintln!("[deep-test] calling drop(api) (planar test)");
    drop(api);
    eprintln!("[deep-test] returned from drop(api) (planar test)");
}

#[test]
#[ignore]
fn test_sound_object_properties_and_room_effects() {
    // Requires native resonance-audio build; enabled by setting RUN_DEEP_INTEGRATION=1
    if std::env::var("RUN_DEEP_INTEGRATION").is_err() {
        eprintln!("skipping deep integration test: set RUN_DEEP_INTEGRATION=1 to run");
        return;
    }
    let api = ResonanceAudioApi::new(2, 256, 48000).expect("Failed to create API");

    let sound_id = api.create_sound_object_source(0);
    assert!(sound_id >= 0);

    eprintln!("[deep-test] calling set_sound_object_directivity");
    api.set_sound_object_directivity(sound_id, 0.5, 1.0);
    eprintln!("[deep-test] calling set_sound_object_listener_directivity");
    api.set_sound_object_listener_directivity(sound_id, 0.2, 0.8);
    eprintln!("[deep-test] calling set_sound_object_near_field_effect_gain");
    api.set_sound_object_near_field_effect_gain(sound_id, 0.3);
    eprintln!("[deep-test] calling set_sound_object_occlusion_intensity");
    api.set_sound_object_occlusion_intensity(sound_id, 0.1);
    eprintln!("[deep-test] calling set_sound_object_spread");
    api.set_sound_object_spread(sound_id, 10.0);

    // Reflection and reverb props: zero-init and set some values
    unsafe {
        let mut refl: ReflectionProperties = mem::zeroed();
        refl.room_position[0] = 1.0;
        refl.room_rotation[3] = 1.0;
    eprintln!("[deep-test] calling set_reflection_properties");
    api.set_reflection_properties(&refl);
    eprintln!("[deep-test] returned from set_reflection_properties");

        let mut reverb: ReverbProperties = mem::zeroed();
        reverb.rt60_values[0] = 0.25;
    eprintln!("[deep-test] calling set_reverb_properties");
    api.set_reverb_properties(&reverb);
    eprintln!("[deep-test] returned from set_reverb_properties");
    }

    eprintln!("[deep-test] calling enable_room_effects");
    api.enable_room_effects(true);
    eprintln!("[deep-test] calling set_master_volume");
    api.set_master_volume(0.9);

    eprintln!("[deep-test] destroying sound source");
    api.destroy_source(sound_id);
    eprintln!("[deep-test] calling drop(api) (sound-object test)");
    drop(api);
    eprintln!("[deep-test] returned from drop(api) (sound-object test)");
}
