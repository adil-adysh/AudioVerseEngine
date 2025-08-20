use resonance_cxx::Api;

#[test]
fn mismatched_channel_lengths_fill_f32() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");
    let mut ch0 = vec![0f32; 64];
    let mut ch1 = vec![0f32; 32]; // mismatched
    let mut channels: Vec<&mut [f32]> = vec![&mut ch0[..], &mut ch1[..]];
    let ok = api.fill_planar_f32(&mut channels[..]);
    assert!(!ok, "expected false due to mismatched channel lengths");
}

#[test]
fn mismatched_channel_lengths_set_i16() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");
    let src = api.create_stereo_source(2);
    let c0 = vec![0i16; 64];
    let c1 = vec![0i16; 32]; // mismatched
    let channels: Vec<&[i16]> = vec![&c0[..], &c1[..]];
    let ok = api.set_planar_buffer_i16(src, &channels[..], 64);
    assert!(!ok, "expected false due to mismatched channel lengths");
}

#[test]
fn zero_channels_fill_set() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");
    let mut empty: Vec<&mut [f32]> = vec![];
    assert!(api.fill_planar_f32(&mut empty[..]));

    let src = api.create_stereo_source(2);
    let empty_in: Vec<&[f32]> = vec![];
    assert!(api.set_planar_buffer_f32(src, &empty_in[..], 0));
}

#[test]
fn large_buffers_set_f32() {
    let mut api = Api::new(8, 1024, 48000).expect("failed to create Api");
    println!("about to create ambisonic source (8 ch)");
    let src = api.create_ambisonic_source(8);
    println!("created source id = {}", src);
    if src < 0 {
        eprintln!("skipping large_buffers_set_f32: ambisonic source creation not supported");
        return;
    }
    // 8 channels, 1024 frames
    let c0 = vec![0.1f32; 1024];
    let c1 = vec![0.2f32; 1024];
    let c2 = vec![0.3f32; 1024];
    let c3 = vec![0.4f32; 1024];
    let c4 = vec![0.5f32; 1024];
    let c5 = vec![0.6f32; 1024];
    let c6 = vec![0.7f32; 1024];
    let c7 = vec![0.8f32; 1024];
    let channels: Vec<&[f32]> = vec![&c0[..], &c1[..], &c2[..], &c3[..], &c4[..], &c5[..], &c6[..], &c7[..]];
    println!("about to call set_planar_buffer_f32");
    let ok = api.set_planar_buffer_f32(src, &channels[..], 1024);
    println!("set_planar_buffer_f32 returned = {}", ok);
    assert!(ok, "large buffer set failed");
}
