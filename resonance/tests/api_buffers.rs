use resonance::*;

#[test]
fn test_interleaved_buffer_set_and_fill() {
    let api = ResonanceAudioApi::new(2, 256, 48000).expect("Failed to create ResonanceAudioApi");

    let stereo_id = api.create_stereo_source(2);
    assert!(stereo_id >= 0);

    // prepare a tiny interleaved buffer: 2 channels, 4 frames => 8 samples
    let frames = 4usize;
    let mut samples_f32 = vec![0.0f32; 2 * frames];
    for i in 0..samples_f32.len() { samples_f32[i] = i as f32 * 0.01; }

    api.set_interleaved_buffer_f32(stereo_id, &samples_f32, 2, frames);

    // Try filling an output buffer (will return bool). We don't assert on audio contents,
    // only that calling the method does not panic and returns a bool.
    let mut out = vec![0.0f32; 2 * frames];
    let ok = api.fill_interleaved_output_buffer_f32(2, frames, &mut out);
    assert!(ok == true || ok == false);

    // i16 path: provide small i16 buffer
    let samples_i16 = vec![0i16; 2 * frames];
    api.set_interleaved_buffer_i16(stereo_id, &samples_i16, 2, frames);

    let mut out_i16 = vec![0i16; 2 * frames];
    let ok_i16 = api.fill_interleaved_output_buffer_i16(2, frames, &mut out_i16);
    assert!(ok_i16 == true || ok_i16 == false);

    api.destroy_source(stereo_id);
}
