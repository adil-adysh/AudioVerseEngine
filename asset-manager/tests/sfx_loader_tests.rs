use asset_manager::sfx_loader::parse_pcm_sfx_data;

#[test]
fn parse_headered_sfx_with_loop() {
    // build header: channels=2, reserved=0, sample_rate=48000, loop start/end, then 4 samples
    let mut data = Vec::new();
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&48000u32.to_le_bytes());
    data.extend_from_slice(&10u64.to_le_bytes()); // loop start
    data.extend_from_slice(&20u64.to_le_bytes()); // loop end
    let samples = [0.1f32, -0.1, 0.2, -0.2];
    for s in &samples {
        data.extend_from_slice(&s.to_le_bytes());
    }
    let (out_samples, meta) = parse_pcm_sfx_data(&data).expect("parse ok");
    assert_eq!(meta.channels, 2);
    assert_eq!(meta.sample_rate, 48000);
    assert!(meta.loop_points.is_some());
    assert_eq!(out_samples.len(), samples.len());
}

#[test]
fn parse_headered_sfx_invalid_header() {
    let data = vec![0u8; 4];
    assert!(parse_pcm_sfx_data(&data).is_err());
}

#[test]
fn fallback_raw_interleaved() {
    // raw f32 bytes with no header; load function should interpret as interleaved f32
    let samples = [0.0f32, 1.0, -1.0, 0.5];
    let mut data = Vec::new();
    for s in &samples {
        data.extend_from_slice(&s.to_le_bytes());
    }
    // parse_pcm_sfx_data should error (no header)
    assert!(parse_pcm_sfx_data(&data).is_err());
}
