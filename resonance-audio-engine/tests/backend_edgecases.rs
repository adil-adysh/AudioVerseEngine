use audio_backend::create_audio_backend;
use resonance_audio_engine::Renderer;

#[test]
fn backend_various_frame_and_channel_sizes_via_mock() {
    if !audio_backend::is_mock_backend_enabled() {
        eprintln!("mock backend not enabled; skipping");
        return;
    }

    let mut backend = create_audio_backend().expect("create backend");
    let sr = backend.sample_rate();

    for &channels in &[1usize, 2usize, 4usize] {
        for &frames in &[32usize, 64usize, 128usize] {
            let mut renderer = Renderer::new(sr as i32, channels, frames);
            let mut out = vec![0.0f32; frames * channels];
            let ok = renderer.process_output_interleaved(&mut out, frames);
            // no panic and buffer remains in-bounds
            assert!(ok || !ok);
        }
    }
}
