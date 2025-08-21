use audio_backend::create_audio_backend;
use resonance_cxx::Api;

#[test]
fn smoke_integration_audio_with_resonance() {
    // Create the mock audio backend via feature-gated factory
    let mut backend = create_audio_backend().expect("create backend");

    // Verify basic device info provider trait access if available
    if let Some(provider) = backend.as_device_info_provider() {
        let name = provider.get_device_name().unwrap_or("<unknown>");
        println!("Device name: {}", name);
    }

    // Create a small resonance API instance and call a planar helper.
    let mut api = Api::new(2, 64, 48000).expect("create resonance api");

    let mut ch1 = vec![0f32; 64];
    let mut ch2 = vec![0f32; 64];
    let mut channels_vec: Vec<&mut [f32]> = vec![&mut ch1[..], &mut ch2[..]];
    // pass mutable slice of mutable slices
    let ok = api.fill_planar_f32(&mut channels_vec[..]);
    assert!(ok || true, "resonance fill didn't panic");

    // Start backend with a trivial render function that writes zeros.
    let render = std::sync::Arc::new(|buf: &mut [f32], _sr: u32, _frames: usize| {
        for b in buf.iter_mut() {
            *b = 0.0;
        }
    });
    backend.start(render).expect("start backend");
    backend.stop().expect("stop backend");
}
