use asset_manager::sfx::SfxBlob;
use asset_manager::util::AssetError;

fn make_sfx_bytes(sf: u8, channels: u8, sample_rate: u32, frames: u64, samples: &[f32]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"SFX1");
    b.push(sf);
    b.push(channels);
    b.extend_from_slice(&[0u8, 0u8]); // reserved
    b.extend_from_slice(&sample_rate.to_le_bytes());
    b.extend_from_slice(&frames.to_le_bytes());
    // append samples as f32 LE
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    b
}

#[test]
fn parse_valid_f32_sfx() {
    let frames = 4u64;
    let channels = 2u8;
    let samples = vec![0.1f32, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8];
    let bytes = make_sfx_bytes(0, channels, 48000u32, frames, &samples);
    let blob = SfxBlob::from_sfx_bytes(&bytes).expect("parse should succeed");
    assert_eq!(blob.sample_rate, 48000);
    assert_eq!(blob.channels, channels as u16);
    assert_eq!(blob.frames, frames);
    assert_eq!(blob.samples.len(), samples.len());
    for (a, b) in blob.samples.iter().zip(samples.iter()) { assert!((a - b).abs() < 1e-6); }
}

#[test]
fn sfx_bad_magic() {
    let mut bytes = make_sfx_bytes(0, 2, 48000, 1, &[0.0f32, 0.0]);
    bytes[0] = b'X';
    match SfxBlob::from_sfx_bytes(&bytes) {
        Err(AssetError::Decode(_)) => {}
        other => panic!("expected decode error, got {:?}", other),
    }
}

#[test]
fn sfx_unknown_sample_format() {
    let bytes = make_sfx_bytes(9, 2, 48000, 1, &[0.0f32, 0.0]);
    match SfxBlob::from_sfx_bytes(&bytes) {
        Err(AssetError::Decode(_)) => {}
        other => panic!("expected decode error, got {:?}", other),
    }
}

#[test]
fn sfx_truncated() {
    let mut bytes = make_sfx_bytes(0, 2, 48000, 2, &[0.1f32, 0.2]);
    bytes.truncate(bytes.len() - 4); // remove part of last sample
    match SfxBlob::from_sfx_bytes(&bytes) {
        Err(AssetError::Decode(_)) => {}
        other => panic!("expected decode error, got {:?}", other),
    }
}

#[test]
fn sfx_zero_frames_rejected() {
    let bytes = make_sfx_bytes(0, 2, 48000, 0, &[]);
    match SfxBlob::from_sfx_bytes(&bytes) {
        Err(AssetError::ResourceLimit(_)) => {}
        other => panic!("expected resource limit error, got {:?}", other),
    }
}
