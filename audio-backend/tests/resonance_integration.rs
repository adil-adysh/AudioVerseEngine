use resonance_ffi::ResonanceCtx;
use audio_backend::{AudioBackend, CpalAudioBackend};

#[test]
fn test_resonance_ctx_creation() {
    let _ctx = ResonanceCtx::new(2, 512, 48000);
    assert!(_ctx.is_ok());
}

#[test]
fn test_audio_backend_with_resonance() {
    let mut backend = CpalAudioBackend::new();
    backend.init().expect("Failed to init backend");
    let ctx = ResonanceCtx::new(2, 512, 48000).expect("Failed to create ResonanceCtx");
    // Generate a buffer (simulate processed audio)
    let buffer = vec![0.1f32; 512 * 2];
    assert!(backend.play(&buffer).is_ok());
    backend.stop().expect("Failed to stop backend");
}

#[test]
fn test_resonance_spatial_sine_wave() {
    let mut backend = CpalAudioBackend::new();
    backend.init().expect("Failed to init backend");
    let ctx = ResonanceCtx::new(2, 512, 48000).expect("Failed to create ResonanceCtx");

    // Generate a sine wave buffer
    let sample_rate = 48000;
    let freq = 440.0;
    let duration_secs = 1.0;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut sine_wave = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * freq * t).sin();
        // Stereo: duplicate sample
        sine_wave.push(sample);
        sine_wave.push(sample);
    }

    // Create a spatial source and set its position
    let source_id = ctx.create_sound_object_source(0).expect("Failed to create source");
    ctx.set_source_position(source_id, 2.0, 0.0, 0.0).expect("Failed to set position");
    ctx.set_source_volume(source_id, 1.0).expect("Failed to set volume");

    // Pass the sine wave buffer to the SDK
    ctx.set_interleaved_buffer_f32(source_id, &sine_wave, 2, num_samples).expect("Failed to set buffer");

    // Retrieve processed output
    let mut output = vec![0.0f32; num_samples * 2];
    ctx.fill_interleaved_output_buffer_f32(2, num_samples, &mut output).expect("Failed to process output");

    // Play the processed buffer
    assert!(backend.play(&output).is_ok());
    backend.stop().expect("Failed to stop backend");

    // Assert output is not identical to input (effect applied)
    assert!(output != sine_wave, "Spatial effect was not applied");
}
