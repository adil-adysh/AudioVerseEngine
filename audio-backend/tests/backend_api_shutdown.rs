use std::time::Duration;
use std::thread::sleep;

use audio_backend::create_audio_backend;
use resonance_cxx::Api;

// This test tries to reproduce the shutdown race by creating/dropping in two orders.
#[test]
fn backend_then_api_drop_order() {
    // Create backend first, then api, then stop/drop
    let mut backend = create_audio_backend().expect("create backend");
    let api = Api::new(2, 128, 48000).expect("create api");

    // Start and stop backend quickly
    let render = std::sync::Arc::new(|buf: &mut [f32], _sr: u32, _frames: usize| {
        for b in buf.iter_mut() { *b = 0.0; }
    });
    backend.start(render).expect("start");
    sleep(Duration::from_millis(100));
    backend.stop().expect("stop");
    std::mem::drop(backend);
    sleep(Duration::from_millis(100));
    drop(api);
}

#[test]
fn api_then_backend_drop_order() {
    // Create api first, then backend, then stop/drop
    let api = Api::new(2, 128, 48000).expect("create api");
    let mut backend = create_audio_backend().expect("create backend");

    let render = std::sync::Arc::new(|buf: &mut [f32], _sr: u32, _frames: usize| {
        for b in buf.iter_mut() { *b = 0.0; }
    });
    backend.start(render).expect("start");
    sleep(Duration::from_millis(100));
    backend.stop().expect("stop");
    std::mem::drop(backend);
    sleep(Duration::from_millis(100));
    drop(api);
}
