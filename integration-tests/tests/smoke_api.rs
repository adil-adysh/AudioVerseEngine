use audio_backend::create_audio_backend;
use resonance_cxx::Api;

#[test]
fn smoke_api_basic() {
    let mut backend = create_audio_backend().expect("create backend");
    if let Some(provider) = backend.as_device_info_provider() {
        let _ = provider.get_device_name();
    }

    let mut api = Api::new(2, 64, 48000).expect("create resonance api");
    let mut ch1 = vec![0f32; 64];
    let mut ch2 = vec![0f32; 64];
    let mut channels_vec: Vec<&mut [f32]> = vec![&mut ch1[..], &mut ch2[..]];
    let _ = api.fill_planar_f32(&mut channels_vec[..]);

    backend
        .start(std::sync::Arc::new(
            |buf: &mut [f32], _sr: u32, _frames: usize| {
                buf.iter_mut().for_each(|b| *b = 0.0);
            },
        ))
        .expect("start");
    backend.stop().expect("stop");
}
