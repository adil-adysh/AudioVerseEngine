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
fn api_full_coverage() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");

    // head/global
    api.set_head_position(1.0, 2.0, 3.0);
    api.set_head_rotation(0.0, 0.0, 0.0, 1.0);
    api.set_master_volume(0.8);
    api.set_stereo_speaker_mode(true);

    // create/destroy sources
    let amb = api.create_ambisonic_source(4);
    let stereo = api.create_stereo_source(2);
    let obj = api.create_sound_object_source(RenderingMode::kBinauralLowQuality);

    // operate on sources when possible
    if amb >= 0 {
        api.set_source_distance_attenuation(amb, 0.5);
        api.set_source_distance_model(amb, DistanceRolloffModel::kLogarithmic, 1.0, 100.0);
        api.set_source_position(amb, 0.0, 0.0, 1.0);
        api.set_source_room_effects_gain(amb, 1.0);
        api.set_source_rotation(amb, 0.0, 0.0, 0.0, 1.0);
        api.set_source_volume(amb, 0.9);
        api.set_sound_object_directivity(amb, 0.5, 1.0);
        api.set_sound_object_listener_directivity(amb, 0.4, 1.0);
        api.set_sound_object_near_field_effect_gain(amb, 0.2);
        api.set_sound_object_occlusion_intensity(amb, 0.1);
        api.set_sound_object_spread(amb, 30.0);
        api.destroy_source(amb);
    }

    if stereo >= 0 {
        // set interleaved buffer (small test)
        let interleaved = vec![0.0f32; 2*64];
        api.set_interleaved_buffer_f32(stereo, &interleaved, 2, 64);
        api.set_interleaved_buffer_i16(stereo, &vec![0i16; 2*64], 2, 64);
        api.destroy_source(stereo);
    }

    if obj >= 0 {
        api.destroy_source(obj);
    }

    // room effects
    api.enable_room_effects(true);
    api.set_reflection_properties(&make_reflection_props());
    api.set_reverb_properties(&make_reverb_props());

    // filling output (interleaved)
    let mut out_f32 = vec![0f32; 2*64];
    let _ = api.fill_interleaved_f32(2, 64, &mut out_f32);
    let mut out_i16 = vec![0i16; 2*64];
    let _ = api.fill_interleaved_i16(2, 64, &mut out_i16);

    // planar helpers
    let mut ch0 = vec![0f32; 64];
    let mut ch1 = vec![0f32; 64];
    let mut planar_mut: Vec<&mut [f32]> = vec![&mut ch0[..], &mut ch1[..]];
    let _ = api.fill_planar_f32(&mut planar_mut[..]);

    let channels_in: Vec<&[f32]> = vec![&ch0[..], &ch1[..]];
    if stereo >= 0 {
        let _ = api.set_planar_buffer_f32(stereo, &channels_in[..], 64);
    }

    // scratch helpers
    if stereo >= 0 {
        let mut scratch = Vec::new();
        let _ = api.set_planar_buffer_f32_with_scratch(stereo, &channels_in[..], 64, &mut scratch);
        let mut scratch_i16: Vec<i16> = Vec::new();
        let _ = api.set_planar_buffer_i16_with_scratch(stereo, &vec![&vec![0i16;64][..], &vec![0i16;64][..]][..], 64, &mut scratch_i16);
    }
}
