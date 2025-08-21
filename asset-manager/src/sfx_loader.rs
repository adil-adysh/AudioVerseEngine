use crate::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SfxMetadata {
    pub channels: u16,
    pub sample_rate: u32,
    pub loop_points: Option<(u64, u64)>,
}

pub(crate) const TARGET_SAMPLE_RATE: u32 = 48000;

/// Load an SFX/PCM file from disk and return interleaved f32 samples plus metadata.
pub fn load_sfx_path(path: &std::path::Path) -> Result<(Vec<f32>, SfxMetadata), Error> {
    let data = std::fs::read(path)?;

    // Try headered parsing first
    if let Ok((samples, meta)) = parse_pcm_sfx_data(&data) {
        // Resample to TARGET_SAMPLE_RATE if needed
        if meta.sample_rate != TARGET_SAMPLE_RATE {
            let samples = resample_interleaved(&samples, meta.sample_rate, TARGET_SAMPLE_RATE, meta.channels as usize);
            let meta = SfxMetadata { sample_rate: TARGET_SAMPLE_RATE, ..meta };
            return Ok((samples, meta));
        }
        return Ok((samples, meta));
    }

    // Fallback: interpret entire blob as raw f32 interleaved
    if data.len() % 4 != 0 {
        return Err(Error::Decode("pcm/sfx data length invalid".into()));
    }

    let mut samples = Vec::with_capacity(data.len() / 4);
    let mut i = 0usize;
    while i + 4 <= data.len() {
        let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
        samples.push(f32::from_le_bytes(b));
        i += 4;
    }

    Ok((samples, SfxMetadata { channels: 2, sample_rate: TARGET_SAMPLE_RATE, loop_points: None }))
}

/// Parse our simple .pcm/.sfx format. Backwards compatible: if only 8-byte header is present
/// we parse channels/sample_rate and samples; if extended header includes loop points (24 bytes)
/// we parse them as well.
pub fn parse_pcm_sfx_data(data: &[u8]) -> Result<(Vec<f32>, SfxMetadata), Error> {
    // Header: [u16 channels][u16 reserved][u32 sample_rate]  => 8 bytes
    // Optional: [u64 loop_start][u64 loop_end] => additional 16 bytes (total 24)
    if data.len() >= 8 {
        let channels = u16::from_le_bytes([data[0], data[1]]);
        let sample_rate = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        let mut loop_points: Option<(u64, u64)> = None;
        if data.len() >= 24 {
            let start = u64::from_le_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]);
            let end = u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]);
            // sanity: only accept loop if start < end
            if start < end {
                loop_points = Some((start, end));
            }
        }

        if channels > 0 && sample_rate > 0 {
            let mut samples = Vec::with_capacity((data.len().saturating_sub(8)) / 4);
            let mut i = 8usize;
            while i + 4 <= data.len() {
                let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                samples.push(f32::from_le_bytes(b));
                i += 4;
            }
            return Ok((samples, SfxMetadata { channels, sample_rate, loop_points }));
        }
    }

    Err(Error::Decode("no headered sfx data".into()))
}

/// Simple linear resampler for interleaved samples. Conservative, single-threaded.
pub(crate) fn resample_interleaved(samples: &[f32], from_rate: u32, to_rate: u32, channels: usize) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let frames = samples.len() / channels;
    let out_frames = ((frames as f64) * ratio).ceil() as usize;
    let mut out = vec![0.0f32; out_frames * channels];

    for ch in 0..channels {
        // gather channel samples
        let mut src: Vec<f32> = Vec::with_capacity(frames);
        for f in 0..frames {
            src.push(samples[f * channels + ch]);
        }

        for t in 0..out_frames {
            let src_pos = (t as f64) / ratio;
            let i0 = src_pos.floor() as isize;
            let i1 = i0 + 1;
            let w = src_pos - (i0 as f64);
            let s0 = if i0 < 0 { 0.0 } else { src.get(i0 as usize).copied().unwrap_or(0.0) };
            let s1 = if i1 < 0 { 0.0 } else { src.get(i1 as usize).copied().unwrap_or(0.0) };
            let val = (1.0 - w) as f32 * s0 + (w as f32) * s1;
            out[t * channels + ch] = val;
        }
    }

    out
}
