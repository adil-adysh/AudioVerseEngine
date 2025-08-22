use resonance_cxx::Api;

#[test]
fn scratch_resize_and_reuse_f32() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");
    let src = api.create_stereo_source(2);
    if src < 0 { eprintln!("skipping scratch test: stereo source not supported"); return; }

    let c0 = vec![0.1f32; 64];
    let c1 = vec![0.2f32; 64];
    let channels: Vec<&[f32]> = vec![&c0[..], &c1[..]];

    let mut scratch: Vec<f32> = Vec::with_capacity(4);
    let ok = api.set_planar_buffer_f32_with_scratch(src, &channels[..], 64, &mut scratch);
    assert!(ok);
    // scratch should have been resized to at least 128
    assert!(scratch.len() >= 2 * 64);

    // Reuse scratch with larger workload - should reuse allocation not crash
    let c0b = vec![0.3f32; 128];
    let c1b = vec![0.4f32; 128];
    let channels2: Vec<&[f32]> = vec![&c0b[..], &c1b[..]];
    let ok2 = api.set_planar_buffer_f32_with_scratch(src, &channels2[..], 128, &mut scratch);
    assert!(ok2);
    assert!(scratch.len() >= 2 * 128);
}

#[test]
fn scratch_resize_and_reuse_i16() {
    let mut api = Api::new(2, 64, 48000).expect("failed to create Api");
    let src = api.create_stereo_source(2);
    if src < 0 { eprintln!("skipping scratch test: stereo source not supported"); return; }

    let c0 = [1i16; 64];
    let c1 = [2i16; 64];
    let channels: Vec<&[i16]> = vec![&c0[..], &c1[..]];

    let mut scratch: Vec<i16> = Vec::with_capacity(4);
    let ok = api.set_planar_buffer_i16_with_scratch(src, &channels[..], 64, &mut scratch);
    assert!(ok);
    assert!(scratch.len() >= 2 * 64);

    let c0b = vec![3i16; 128];
    let c1b = vec![4i16; 128];
    let channels2: Vec<&[i16]> = vec![&c0b[..], &c1b[..]];
    let ok2 = api.set_planar_buffer_i16_with_scratch(src, &channels2[..], 128, &mut scratch);
    assert!(ok2);
    assert!(scratch.len() >= 2 * 128);
}
