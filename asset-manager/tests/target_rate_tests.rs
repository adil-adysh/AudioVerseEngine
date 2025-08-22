use tempfile::tempdir;
use std::fs::File;
use std::io::Write;

fn make_headered_pcm(channels: u16, sample_rate: u32, samples: &[f32]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&sample_rate.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&s.to_le_bytes());
    }
    v
}

#[test]
fn explicit_target_rate_returns_requested_rate() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("tgt.sfx");
    let samples = vec![0.0f32, 0.5, -0.5, 1.0]; // 2 frames stereo
    let data = make_headered_pcm(2, 24000, &samples);
    let mut f = File::create(&p).unwrap();
    f.write_all(&data).unwrap();

    let (out1, meta1) = asset_manager::sfx_loader::load_sfx_path_with_target(&p, 32000).unwrap();
    eprintln!("out1.len={} samples.len={} meta1={:?}", out1.len(), samples.len(), meta1);
    assert_eq!(meta1.sample_rate, 32000);
    assert!(out1.len() > samples.len(), "out1.len={} samples.len={} meta1={:?}", out1.len(), samples.len(), meta1);

    let (out2, meta2) = asset_manager::sfx_loader::load_sfx_path_with_target(&p, 48000).unwrap();
    eprintln!("out2.len={} samples.len={} meta2={:?}", out2.len(), samples.len(), meta2);
    assert_eq!(meta2.sample_rate, 48000);
    assert!(out2.len() > samples.len(), "out2.len={} samples.len={} meta2={:?}", out2.len(), samples.len(), meta2);
}
