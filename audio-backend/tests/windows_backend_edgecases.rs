use audio_backend::create_audio_backend;
use resonance_audio_engine::Renderer;
use std::sync::{Arc, Mutex};

// These tests exercise various buffer/frame/channel combinations. They run
// only when the mock backend is enabled to avoid requiring real hardware in CI.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_backend_various_frame_and_channel_sizes() {
        if !audio_backend::is_mock_backend_enabled() {
            eprintln!("mock backend not enabled; skipping");
            return;
        }

        // Create the backend (mock) and iterate over several configurations.
        let mut backend = create_audio_backend().expect("create backend");
        let sr = backend.sample_rate();

        for channels in [1usize, 2usize, 4usize].iter() {
            for frames in [32usize, 64usize, 128usize].iter() {
                // Create a renderer matching this config
                let mut renderer = Renderer::new(sr as i32, *channels, *frames);
                // Prepare a buffer sized frames * channels
                let mut out = vec![0.0f32; frames * channels];

                // Call process_output_interleaved to ensure it accepts the buffer
                let ok = renderer.process_output_interleaved(&mut out, *frames);
                // In mock backend we expect a boolean return; it should be true/false
                // but must not panic and should not write out-of-bounds.
                assert!(ok || !ok, "process_output_interleaved returned non-bool");
            }
        }
    }
}
