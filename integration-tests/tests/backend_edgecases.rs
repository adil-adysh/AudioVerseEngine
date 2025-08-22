use audio_backend::create_audio_backend;
use resonance_audio_engine::Renderer;

#[test]
fn backend_edgecases_various_frames_channels() {
    if !audio_backend::is_mock_backend_enabled() {
        eprintln!("mock backend not enabled; skipping");
        return;
    }

    let mut backend = create_audio_backend().expect("create backend");
    let sr = backend.sample_rate();

    for &channels in &[1usize, 2usize, 4usize] {
        for &frames in &[16usize, 32usize, 48usize, 64usize, 128usize] {
            let mut renderer = Renderer::new(sr as i32, channels, frames);
            let mut out = vec![0.0f32; frames * channels];
            // ensure no panic and process call returns bool
            let _ = renderer.process_output_interleaved(&mut out, frames);
        }
    }
}
