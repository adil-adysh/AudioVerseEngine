#[cfg(feature = "streaming")]
use ringbuf::{HeapRb, HeapProd, HeapCons};
#[cfg(feature = "streaming")]
use ringbuf::traits::{Split, Producer, Consumer};
#[cfg(feature = "streaming")]
use std::thread;
#[cfg(feature = "streaming")]
use rubato::Resampler;

#[cfg(feature = "streaming")]
pub struct StreamingAsset {
    consumer: ringbuf::HeapCons<f32>,
    _handle: thread::JoinHandle<()>,
}

#[cfg(feature = "streaming")]
impl StreamingAsset {
    pub fn open(path: &str) -> Result<StreamingAsset, String> {
    let rb = HeapRb::<f32>::new(32 * 1024);
    let (mut prod, cons) = rb.split();
        let path_str = path.to_string();

        let handle = thread::spawn(move || {
            let mut resampler: Option<rubato::SincFixedIn<f32>> = None;
            let mut resampler_ratio: Option<f64> = None;

            let ext = std::path::Path::new(&path_str)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase());

            // direct PCM/SFX read
            if ext.as_deref() == Some("pcm") || ext.as_deref() == Some("sfx") {
                if let Ok(data) = std::fs::read(&path_str) {
                    let mut i = 8usize;
                    while i + 4 <= data.len() {
                        let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                        let _ = prod.try_push(f32::from_le_bytes(b));
                        i += 4;
                    }
                }
                return;
            }

            // decode other formats
            if let Err(e) = (|| -> Result<(), String> {
                use symphonia::core::{
                    audio::{AudioBufferRef, SampleBuffer},
                    codecs::DecoderOptions,
                    formats::FormatOptions,
                    io::MediaSourceStream,
                    meta::MetadataOptions,
                };
                use symphonia::default::{get_codecs, get_probe};

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
                            // Normalize any decoded buffer into f32 interleaved samples
                            let spec = audio_buf.spec();
                            let mut sample_buf = SampleBuffer::<f32>::new(
                                audio_buf.capacity() as u64,
                                *spec,
                            );
                            sample_buf.copy_interleaved_ref(audio_buf);
                            let sr = spec.rate as u32;
                            let channels = spec.channels.count();
                            let samples_vec = sample_buf.samples().to_vec();

                            // same sample rate, push directly
                            if sr == crate::sfx_loader::TARGET_SAMPLE_RATE {
                                push_in_chunks(&mut prod, &samples_vec);
                                continue;
                            }

                            // resample path
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
                                // rubato::Resampler trait is imported; call process directly
                                match r.process(&planar.iter().map(|v| v.as_slice()).collect::<Vec<_>>(), None) {
                                    Ok(outputs) => {
                                        if outputs.is_empty() {
                                            continue;
                                        }
                                        let interleaved = interleave(&outputs, channels);
                                        push_in_chunks(&mut prod, &interleaved);
                                    }
                                    Err(e) => {
                                        eprintln!("resample process error: {:?}", e);
                                        continue;
                                    }
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
        self.consumer.pop_slice(out)
    }
}

/// Push data into ring buffer in 1024-sample chunks
#[cfg(feature = "streaming")]
fn push_in_chunks(prod: &mut ringbuf::HeapProd<f32>, data: &[f32]) {
    let mut off = 0usize;
    while off < data.len() {
        let end = (off + 1024).min(data.len());
    let _ = prod.push_slice(&data[off..end]);
        off = end;
    }
}

/// Convert interleaved samples into planar channel layout
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

/// Convert rubato's planar output back to interleaved samples
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

/// Ensure resampler is initialized or updated when ratio changes
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
        use rubato::{InterpolationParameters, InterpolationType, SincFixedIn, WindowFunction};
        let params = InterpolationParameters {
            sinc_len: 128,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Cubic,
            oversampling_factor: 32,
            window: WindowFunction::BlackmanHarris2,
        };
        let chunk_size = frames.max(1024);
        let cutoff_scale: f64 = 0.95;
        *resampler = Some(
            SincFixedIn::<f32>::new(ratio, cutoff_scale, params, channels, chunk_size)
                .expect("failed to create rubato resampler"),
        );
        *resampler_ratio = Some(ratio);
    }
}
