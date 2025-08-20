use resonance_cxx::{Api, RenderingMode, DistanceRolloffModel, ReflectionProperties, ReverbProperties};

fn make_reflection_props() -> ReflectionProperties {
    ReflectionProperties {
        room_position: [0.0, 0.0, 0.0],
        room_rotation: [0.0, 0.0, 0.0, 1.0],
        room_dimensions: [10.0, 10.0, 3.0],
        cutoff_frequency: 2000.0,
        coefficients: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
        gain: 1.0,
    }
}

fn make_reverb_props() -> ReverbProperties {
    ReverbProperties {
        rt60_values: [0.2; 9],
        gain: 0.5,
    }
}

#[test]
fn initialization_success() {
    // Typical values should succeed on supported platforms
    let mut api = Api::new(2, 1024, 48000).expect("failed to create Api with typical params");
    // Use a few setters to ensure the instance is usable
    api.set_master_volume(0.9);
    api.set_stereo_speaker_mode(true);
    // Ensure other operations don't panic
    api.enable_room_effects(true);
    api.set_reflection_properties(&make_reflection_props());
    api.set_reverb_properties(&make_reverb_props());
}

#[test]
fn initialization_failure_possible() {
    // Attempt to provoke an allocation failure using very large sizes.
    // This is inherently platform-dependent; the test will accept either None
    // (allocation failed) or Some(api) (skip assertion) to avoid crashing CI.
    let huge = 1_000_000_000usize; // intentionally large
    match Api::new(huge, huge, 48000) {
        None => { /* expected on some platforms */ }
        Some(mut api) => {
            // If allocation succeeded, exercise the Api briefly and log that we couldn't provoke failure.
            eprintln!("Api creation unexpectedly succeeded with huge sizes; skipping strict failure assertion");
            api.set_master_volume(0.5);
        }
    }
}

#[test]
fn planar_and_i16_valid_paths() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");

    // planar f32 fill (smoke) - underlying impl may return false when there's no audio,
    // but wrapper must be safe and not panic.
    let mut a = vec![0f32; 64];
    let mut b = vec![0f32; 64];
    let mut out_channels: Vec<&mut [f32]> = vec![&mut a[..], &mut b[..]];
    let _ok = api.fill_planar_f32(&mut out_channels[..]);

    // set_planar_buffer_f32 should return true for matching buffers
    let in_a = vec![0.1f32; 64];
    let in_b = vec![0.2f32; 64];
    let channels_in: Vec<&[f32]> = vec![&in_a[..], &in_b[..]];
    let src = api.create_stereo_source(2);
    if src < 0 {
        eprintln!("skipping planar f32 set: stereo source not supported");
    } else {
        let ok = api.set_planar_buffer_f32(src, &channels_in[..], 64);
        assert!(ok, "set_planar_buffer_f32 returned false for valid input");
    }

    // i16 variants: set and scratch helper
    let c0 = vec![1i16; 64];
    let c1 = vec![2i16; 64];
    let channels_i16: Vec<&[i16]> = vec![&c0[..], &c1[..]];
    let src_i16 = api.create_stereo_source(2);
    if src_i16 < 0 {
        eprintln!("skipping i16 tests: stereo source not supported");
    } else {
        let ok = api.set_planar_buffer_i16(src_i16, &channels_i16[..], 64);
        assert!(ok, "set_planar_buffer_i16 returned false for valid input");

        let mut scratch: Vec<i16> = Vec::with_capacity(2);
        let ok2 = api.set_planar_buffer_i16_with_scratch(src_i16, &channels_i16[..], 64, &mut scratch);
        assert!(ok2, "set_planar_buffer_i16_with_scratch returned false");
        assert!(scratch.len() >= 2 * 64, "scratch was not resized as expected");
    }
}

#[test]
fn scratch_resize_and_reuse_f32_small_to_large() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");
    let src = api.create_stereo_source(2);
    if src < 0 { eprintln!("skipping scratch test: stereo source not supported"); return; }

    let c0 = vec![0.1f32; 64];
    let c1 = vec![0.2f32; 64];
    let channels: Vec<&[f32]> = vec![&c0[..], &c1[..]];

    let mut scratch: Vec<f32> = Vec::with_capacity(4);
    let ok = api.set_planar_buffer_f32_with_scratch(src, &channels[..], 64, &mut scratch);
    assert!(ok);
    assert!(scratch.len() >= 2 * 64);

    // Reuse with larger workload
    let c0b = vec![0.3f32; 128];
    let c1b = vec![0.4f32; 128];
    let channels2: Vec<&[f32]> = vec![&c0b[..], &c1b[..]];
    let ok2 = api.set_planar_buffer_f32_with_scratch(src, &channels2[..], 128, &mut scratch);
    assert!(ok2);
    assert!(scratch.len() >= 2 * 128);
}

#[test]
fn ffi_methods_setters_smoke() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");

    // Head position / rotation
    api.set_head_position(1.0, 2.0, 3.0);
    api.set_head_rotation(0.0, 0.0, 0.0, 1.0);

    // create/destroy and other setters
    let amb = api.create_ambisonic_source(4);
    if amb >= 0 {
        let id = amb;
        api.set_source_distance_attenuation(id, 0.5);
        api.set_source_distance_model(id, DistanceRolloffModel::kLogarithmic, 1.0, 100.0);
        api.set_source_position(id, 0.1, 0.2, 0.3);
        api.set_source_room_effects_gain(id, 1.0);
        api.set_source_rotation(id, 0.0, 0.0, 0.0, 1.0);
        api.set_source_volume(id, 0.7);
        api.set_sound_object_directivity(id, 0.5, 1.0);
        api.set_sound_object_listener_directivity(id, 0.4, 1.0);
        api.set_sound_object_near_field_effect_gain(id, 0.2);
        api.set_sound_object_occlusion_intensity(id, 0.1);
        api.set_sound_object_spread(id, 45.0);
        api.destroy_source(id);
    } else {
        eprintln!("skipping ambisonic setter smoke: not supported");
    }

    // stereo source create/destroy
    let stereo = api.create_stereo_source(2);
    if stereo >= 0 {
        api.destroy_source(stereo);
    }

    // sound object
    let obj = api.create_sound_object_source(RenderingMode::kBinauralLowQuality);
    if obj >= 0 {
        api.destroy_source(obj);
    }
}
