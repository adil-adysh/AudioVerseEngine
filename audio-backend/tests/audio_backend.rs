use audio_backend::{AudioBackend, CpalAudioBackend};

#[test]
fn test_backend_init() {
    let mut backend = CpalAudioBackend::new();
    assert!(backend.init().is_ok());
}

#[test]
fn test_backend_play_stop() {
    let mut backend = CpalAudioBackend::new();
    backend.init().expect("Failed to init backend");
    let buffer = vec![0.0f32; 48000]; // 1 second of silence at 48kHz
    assert!(backend.play(&buffer).is_ok());
    assert!(backend.stop().is_ok());
}
