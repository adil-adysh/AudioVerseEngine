use resonance_cxx::Api;

#[test]
fn planar_smoke() {
    // Small configuration for smoke test
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");

    // Prepare planar buffers (2 channels, 64 frames)
    let mut ch0 = vec![0f32; 64];
    let mut ch1 = vec![0f32; 64];
    let mut channels: Vec<&mut [f32]> = vec![&mut ch0[..], &mut ch1[..]];

    // Attempt to fill planar output via the helper. The underlying
    // implementation may return false if no audio was produced; this
    // smoke test ensures the crossing is safe and does not panic or
    // corrupt buffer lengths.
    let _ok = api.fill_planar_f32(&mut channels[..]);
    assert_eq!(channels[0].len(), 64);
    assert_eq!(channels[1].len(), 64);

    // Create a stereo source and set its planar buffer
    let src = api.create_stereo_source(2);
    let c0 = vec![0.0f32; 64];
    let c1 = vec![0.0f32; 64];
    let channels_in: Vec<&[f32]> = vec![&c0[..], &c1[..]];
    let set_ok = api.set_planar_buffer_f32(src, &channels_in[..], 64);
    assert!(set_ok, "set_planar_buffer_f32 returned false");
}
