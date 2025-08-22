use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
const TARGET_SAMPLE_RATE: u32 = 48000;

fn make_headered_pcm(channels: u16, sample_rate: u32, samples: &[f32]) -> Vec<u8> {
    // minimal 8-byte header then interleaved f32 samples
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
fn load_sfx_no_resample_when_same_rate() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("t.sfx");
    let samples = vec![0.0f32, 1.0, -1.0, 0.5]; // 2 frames stereo
    let data = make_headered_pcm(2, TARGET_SAMPLE_RATE, &samples);
    let mut f = File::create(&p).unwrap();
    f.write_all(&data).unwrap();
    let (out, meta) =
        asset_manager::sfx_loader::load_sfx_path_with_target(&p, TARGET_SAMPLE_RATE).unwrap();
    assert_eq!(meta.sample_rate, TARGET_SAMPLE_RATE);
    assert_eq!(out, samples);
}

#[test]
fn load_sfx_resamples_if_needed() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("t2.sfx");
    let samples = vec![0.0f32, 0.0, 1.0, 1.0]; // 2 frames stereo
                                               // write with 24000 -> will be resampled to TARGET_SAMPLE_RATE
    let data = make_headered_pcm(2, 24000, &samples);
    let mut f = File::create(&p).unwrap();
    f.write_all(&data).unwrap();
    let (out, meta) =
        asset_manager::sfx_loader::load_sfx_path_with_target(&p, TARGET_SAMPLE_RATE).unwrap();
    assert_eq!(meta.sample_rate, TARGET_SAMPLE_RATE);
    assert!(out.len() != samples.len());
}
