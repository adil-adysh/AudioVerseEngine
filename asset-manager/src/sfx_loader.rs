use crate::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SfxMetadata {
    pub channels: u16,
    pub sample_rate: u32,
    pub loop_points: Option<(u64, u64)>,
}

pub const TARGET_SAMPLE_RATE: u32 = 48000;

// Note: `load_sfx_path` was removed. Use `load_sfx_path_with_target(path, target_sample_rate)` instead.
/// Load an SFX and ensure the returned samples are at `target_sample_rate`.
pub fn load_sfx_path_with_target(
    path: &std::path::Path,
    target_sample_rate: u32,
) -> Result<(Vec<f32>, SfxMetadata), Error> {
    let data = std::fs::read(path)?;

    if let Ok((samples, meta)) = parse_pcm_sfx_data(&data) {
        if meta.sample_rate != target_sample_rate {
            let samples = resample_interleaved(
                &samples,
                meta.sample_rate,
                target_sample_rate,
                meta.channels as usize,
            );
            let meta = SfxMetadata {
                sample_rate: target_sample_rate,
                ..meta
            };
            return Ok((samples, meta));
        }
        return Ok((samples, meta));
    }

    // Fallback: raw interleaved f32
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
    // if target differs from default, resample
    let final_samples = if target_sample_rate != TARGET_SAMPLE_RATE {
        resample_interleaved(&samples, TARGET_SAMPLE_RATE, target_sample_rate, 2)
    } else {
        samples
    };

    Ok((
        final_samples,
        SfxMetadata {
            channels: 2,
            sample_rate: target_sample_rate,
            loop_points: None,
        },
    ))
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
        // Determine header size: 8 bytes base, optional 16 bytes for loop points
        let mut header_size = 8usize;
        if data.len() >= 24 {
            let start = u64::from_le_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]);
            let end = u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]);
            // sanity: only accept loop if start < end AND there is at least one sample following the extended header.
            // This prevents misinterpreting a blob that is exactly 8-byte header + 16 bytes of samples (total 24)
            // as an extended header with no samples.
            if start < end && data.len() > 24 {
                loop_points = Some((start, end));
                header_size = 24usize;
            }
        }

        if channels > 0 && sample_rate > 0 {
            let mut samples = Vec::with_capacity((data.len().saturating_sub(header_size)) / 4);
            let mut i = header_size;
            while i + 4 <= data.len() {
                let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                samples.push(f32::from_le_bytes(b));
                i += 4;
            }
            return Ok((
                samples,
                SfxMetadata {
                    channels,
                    sample_rate,
                    loop_points,
                },
            ));
        }
    }

    Err(Error::Decode("no headered sfx data".into()))
}

pub fn resample_interleaved(
    samples: &[f32],
    from_rate: u32,
    to_rate: u32,
    channels: usize,
) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }

    // Use rubato crate's SincFixedIn resampler for high-quality resampling.
    // rubato expects planar (per-channel) frames in Vec<Vec<f32>> with same length per channel.
    // Convert interleaved -> planar, resample, then reconvert.
    use rubato::{
        InterpolationParameters, InterpolationType, Resampler, SincFixedIn, WindowFunction,
    };

    let frames = samples.len() / channels;
    let ratio = to_rate as f64 / from_rate as f64;

    // For very small frames, avoid creating a heavy resampler; use simple nearest-neighbor resampling.
    if frames == 0 {
        return Vec::new();
    }
    if frames < 16 {
        let out_frames = ((frames as f64) * ratio).ceil().max(1.0) as usize;
        let mut out = vec![0.0f32; out_frames * channels];
        for f in 0..out_frames {
            let src_f = ((f as f64) / ratio).floor() as usize;
            let src_f = std::cmp::min(src_f, frames - 1);
            for ch in 0..channels {
                out[f * channels + ch] = samples[src_f * channels + ch];
            }
        }
        return out;
    }

    // InterpolationParameters fields: sinc_len, f_cutoff, interpolation, oversampling_factor, window
    let params = InterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: InterpolationType::Cubic,
        oversampling_factor: 32,
        window: WindowFunction::BlackmanHarris2,
    };
    let chunk_size = frames.max(1024);

    // build planar vectors
    let mut planar: Vec<Vec<f32>> = vec![Vec::with_capacity(frames); channels];
    for f in 0..frames {
        for ch in 0..channels {
            planar[ch].push(samples[f * channels + ch]);
        }
    }

    // SincFixedIn::new expected signature includes an extra f64 parameter (filter cutoff scaling)
    // and expects channels before chunk_size in this rubato version.
    let cutoff_scale: f64 = 0.95;
    let max_ratio = if ratio < 1.0 { 1.0 } else { ratio };
    // rubato's SincFixedIn in this version expects chunk_size before channels.
    let mut resampler =
        SincFixedIn::<f32>::new(cutoff_scale, max_ratio, params, chunk_size, channels)
            .expect("failed to create rubato resampler");

    // rubato expects slices: &[&[f32]] per chunk
    let input_refs: Vec<&[f32]> = planar.iter().map(|v| v.as_slice()).collect();
    let outputs = resampler
        .process(&input_refs, None)
        .expect("rubato resample failed");

    // outputs is Vec<Vec<f32>> per channel
    if outputs.is_empty() {
        return Vec::new();
    }
    let out_frames = outputs[0].len();
    let mut out = vec![0.0f32; out_frames * channels];
    for f in 0..out_frames {
        for ch in 0..channels {
            out[f * channels + ch] = outputs[ch][f];
        }
    }
    out
}
