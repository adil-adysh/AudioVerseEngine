use std::f32::consts::PI;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crate::{AudioBackend, CpalAudioBackend};

/// Simple test: play a 440Hz sine wave for 2 seconds.
pub fn test_play_sine() {
    let mut backend = CpalAudioBackend::new();
    backend.init().expect("Failed to init audio backend");
    let sample_rate = 48000;
    let freq = 440.0;
    let duration_secs = 2.0;
    let frames = (sample_rate as f32 * duration_secs) as usize;
    let mut buffer = vec![0.0f32; frames * 2]; // stereo
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * freq * t).sin() * 0.2;
        buffer[i * 2] = sample;      // left
        buffer[i * 2 + 1] = sample;  // right
    }
    backend.play(&buffer).expect("Failed to play buffer");
    thread::sleep(Duration::from_secs_f32(duration_secs));
    backend.stop().expect("Failed to stop playback");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sine_playback() {
        test_play_sine();
    }
}
