// Feature-gated streaming loader. When the `streaming` feature is enabled this
// file exposes `StreamingAsset` which decodes audio on a worker thread and
// provides a consumer-side API to read interleaved f32 samples from a ring
// buffer. The implementation is intentionally minimal and self-contained so it
// compiles cleanly under clippy while preserving the existing decoding and
// resampling logic.

#[cfg(feature = "streaming")]
use std::thread;

#[cfg(feature = "streaming")]
use ringbuf::traits::{Consumer, Producer, Split};
#[cfg(feature = "streaming")]
use ringbuf::{HeapCons, HeapProd, HeapRb};

#[cfg(feature = "streaming")]
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions,
};
#[cfg(feature = "streaming")]
use symphonia::default::{get_codecs, get_probe};

#[cfg(feature = "streaming")]
pub struct StreamingAsset {
    consumer: HeapCons<f32>,
    _handle: thread::JoinHandle<()>,
}

#[cfg(feature = "streaming")]
impl StreamingAsset {
    pub fn open(path: &str) -> Result<StreamingAsset, String> {
        let rb = HeapRb::<f32>::new(32 * 1024);
        // split requires the Split trait in scope
        let (mut prod, cons) = rb.split();
        let path_str = path.to_string();

        let handle = thread::spawn(move || {
            // Worker: decode common formats into f32 and push into ringbuf.
            let mut resampler: Option<rubato::SincFixedIn<f32>> = None;
            let mut resampler_ratio: Option<f64> = None;

            let ext = std::path::Path::new(&path_str)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase());

            // Simple raw formats: PCM-like or our internal .sfx layout.
            if ext.as_deref() == Some("pcm") || ext.as_deref() == Some("sfx") {
                if let Ok(data) = std::fs::read(&path_str) {
                    // skip a small header if present (keeps parity with previous loader)
                    let mut i = 8usize;
                    while i + 4 <= data.len() {
                        let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                        let _ = prod.try_push(f32::from_le_bytes(b));
                        i += 4;
                    }
                }
                return;
            }

            // Use symphonia to decode other formats; errors are logged but do not
            // panic the worker thread.
            if let Err(e) = (|| -> Result<(), String> {
                let file = std::fs::File::open(&path_str).map_err(|e| format!("open: {}", e))?;
                let mss = MediaSourceStream::new(Box::new(file), Default::default());
                let probed = get_probe()
                    .format(
                        &Default::default(),
                        mss,
                        &FormatOptions::default(),
                        &MetadataOptions::default(),
                    )
                    .map_err(|e| format!("probe error: {}", e))?;

                let mut format = probed.format;
                let track = format
                    .default_track()
                    .ok_or_else(|| "no default track".to_string())?;
                let mut decoder = get_codecs()
                    .make(&track.codec_params, &DecoderOptions::default())
                    .map_err(|e| format!("codec make error: {}", e))?;

                while let Ok(packet) = format.next_packet() {
                    match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            // Convert to interleaved f32 samples.
                            let spec = *audio_buf.spec();
                            let mut sample_buf =
                                SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
                            sample_buf.copy_interleaved_ref(audio_buf);
                            let sr = spec.rate;
                            let channels = spec.channels.count();
                            let samples_vec = sample_buf.samples().to_vec();

                            if sr == crate::sfx_loader::TARGET_SAMPLE_RATE {
                                push_in_chunks(&mut prod, &samples_vec);
                                continue;
                            }

                            let ratio = crate::sfx_loader::TARGET_SAMPLE_RATE as f64 / sr as f64;
                            let frames = samples_vec.len() / channels;
                            let planar = to_planar(&samples_vec, channels);

                            ensure_resampler(
                                &mut resampler,
                                &mut resampler_ratio,
                                ratio,
                                channels,
                                frames,
                            );

                            if let Some(r) = resampler.as_mut() {
                                // rubato's processing trait is required for `process`.
                                use rubato::Resampler;
                                if let Ok(outputs) = r.process(
                                    &planar.iter().map(|v| v.as_slice()).collect::<Vec<_>>(),
                                    None,
                                ) {
                                    if outputs.is_empty() {
                                        continue;
                                    }
                                    let interleaved = interleave(&outputs, channels);
                                    push_in_chunks(&mut prod, &interleaved);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }

                Ok(())
            })() {
                eprintln!("stream decode err: {}", e);
            }
        });

        Ok(StreamingAsset {
            consumer: cons,
            _handle: handle,
        })
    }

    pub fn read(&mut self, out: &mut [f32]) -> usize {
        // Consumer::pop_slice is provided by the trait in scope
        self.consumer.pop_slice(out)
    }
}

#[cfg(feature = "streaming")]
fn push_in_chunks(prod: &mut HeapProd<f32>, data: &[f32]) {
    let mut off = 0usize;
    while off < data.len() {
        let end = (off + 1024).min(data.len());
        // Ignore failures; consumer may be slow and partial writes are fine.
        let _ = prod.push_slice(&data[off..end]);
        off = end;
    }
}

#[cfg(feature = "streaming")]
fn to_planar(samples: &[f32], channels: usize) -> Vec<Vec<f32>> {
    let frames = samples.len() / channels;
    let mut planar: Vec<Vec<f32>> = vec![Vec::with_capacity(frames); channels];
    for f in 0..frames {
        for ch in 0..channels {
            planar[ch].push(samples[f * channels + ch]);
        }
    }
    planar
}

#[cfg(feature = "streaming")]
fn interleave(outputs: &[Vec<f32>], channels: usize) -> Vec<f32> {
    let out_frames = outputs[0].len();
    let mut interleaved = vec![0.0f32; out_frames * channels];
    for f in 0..out_frames {
        for ch in 0..channels {
            interleaved[f * channels + ch] = outputs[ch][f];
        }
    }
    interleaved
}

#[cfg(feature = "streaming")]
fn ensure_resampler(
    resampler: &mut Option<rubato::SincFixedIn<f32>>,
    resampler_ratio: &mut Option<f64>,
    ratio: f64,
    channels: usize,
    frames: usize,
) {
    let recreate = match resampler_ratio {
        Some(r) => (*r - ratio).abs() > 1e-8,
        None => true,
    };

    if recreate {
    use rubato::{SincInterpolationParameters, SincInterpolationType, SincFixedIn, WindowFunction};
        let params = InterpolationParameters {
            sinc_len: 128,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Cubic,
            oversampling_factor: 32,
            window: WindowFunction::BlackmanHarris2,
        };
        let chunk_size = frames.max(1024);
        let cutoff_scale: f64 = 0.95;
        let max_ratio = if ratio < 1.0 { 1.0 } else { ratio };
        *resampler = Some(
            // rubato expects (f_cutoff, max_resample_ratio_relative, params, chunk_size, channels)
            SincFixedIn::<f32>::new(cutoff_scale, max_ratio, params, chunk_size, channels)
                .expect("failed to create rubato resampler"),
        );
        *resampler_ratio = Some(ratio);
    }
}
