use std::path::Path;

const TARGET_SAMPLE_RATE: u32 = 48000;

pub fn convert_to_sfx_bytes(path: &Path) -> anyhow::Result<Vec<u8>> {
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::default::get_probe;

    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = get_probe().format(
        &Default::default(),
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("no default track"))?;
    let sr = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow::anyhow!("sample rate unknown"))?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as usize;

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(_) => break,
        };
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                // Convert decoded audio to interleaved f32 using SampleBuffer
                use symphonia::core::audio::SampleBuffer;
                let spec = *audio_buf.spec();
                let mut sample_buf = SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
                sample_buf.copy_interleaved_ref(audio_buf);
                let s = sample_buf.samples();
                samples.extend_from_slice(s);
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    let out_samples = if sr != TARGET_SAMPLE_RATE {
        resample_interleaved(&samples, sr, TARGET_SAMPLE_RATE, channels)
    } else {
        samples
    };

    let bytes = write_sfx_bytes(&out_samples, TARGET_SAMPLE_RATE, channels as u16)?;
    Ok(bytes)
}

fn resample_interleaved(
    samples: &[f32],
    from_rate: u32,
    to_rate: u32,
    channels: usize,
) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    use rubato::{
        SincInterpolationParameters, SincInterpolationType, Resampler, SincFixedIn, WindowFunction,
    };

    let frames = samples.len() / channels;
    let ratio = to_rate as f64 / from_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 32,
        window: WindowFunction::BlackmanHarris2,
    };
    let chunk_size = frames.max(1024);

    let mut planar: Vec<Vec<f32>> = vec![Vec::with_capacity(frames); channels];
    for f in 0..frames {
        for ch in 0..channels {
            planar[ch].push(samples[f * channels + ch]);
        }
    }

    let cutoff_scale: f64 = 0.95;
    // SincFixedIn in this rubato version expects the f_cutoff parameter first,
    // then the max_resample_ratio_relative. Also ensure the max ratio is >= 1.0.
    let max_ratio = if ratio < 1.0 { 1.0 } else { ratio };
    // rubato's SincFixedIn in this version expects chunk_size before channels.
    let mut resampler = SincFixedIn::<f32>::new(cutoff_scale, max_ratio, params, chunk_size, channels)
        .expect("failed to create resampler");
    let input_refs: Vec<&[f32]> = planar.iter().map(|v| v.as_slice()).collect();
    let outputs = resampler
        .process(&input_refs, None)
        .expect("resample failed");

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

fn write_sfx_bytes(samples: &[f32], sample_rate: u32, channels: u16) -> anyhow::Result<Vec<u8>> {
    let mut v: Vec<u8> = Vec::with_capacity(20 + samples.len() * 4);
    v.extend_from_slice(b"SFX1");
    v.push(0u8); // F32
    v.push(channels as u8);
    v.push(0u8);
    v.push(0u8);
    v.extend_from_slice(&sample_rate.to_le_bytes());
    let frames: u64 = (samples.len() / channels as usize) as u64;
    v.extend_from_slice(&frames.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&s.to_le_bytes());
    }
    Ok(v)
}
