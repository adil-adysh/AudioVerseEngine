use crate::Error;

#[cfg(feature = "streaming")]
use ringbuf::{HeapRb, traits::Producer};
#[cfg(feature = "streaming")]
use std::thread;

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
            // Try symphonia decoding first; fall back to naive read for .pcm/.sfx
            let ext = std::path::Path::new(&path_str).extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase());
            if ext.as_deref() == Some("pcm") || ext.as_deref() == Some("sfx") {
                if let Ok(data) = std::fs::read(&path_str) {
                    let mut i = 8usize;
                    while i + 4 <= data.len() {
                        let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                        let _ = prod.push(f32::from_le_bytes(b));
                        i += 4;
                    }
                }
                return;
            }

            // Use symphonia for other formats
            if let Err(e) = (|| -> Result<(), String> {
                use symphonia::default::{get_probe, get_codecs};
                use symphonia::core::io::MediaSourceStream;
                use symphonia::core::formats::FormatOptions;
                use symphonia::core::meta::MetadataOptions;
                use symphonia::core::codecs::DecoderOptions;
                use symphonia::core::audio::{AudioBufferRef, SampleBuffer};

                let file = std::fs::File::open(&path_str).map_err(|e| format!("open: {}", e))?;
                let mss = MediaSourceStream::new(Box::new(file), Default::default());
                let probed = get_probe()
                    .format(&Default::default(), mss, &FormatOptions::default(), &MetadataOptions::default())
                    .map_err(|e| format!("probe error: {}", e))?;

                let mut format = probed.format;
                let track = format.default_track().ok_or_else(|| "no default track".to_string())?;
                let mut decoder = get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| format!("codec make error: {}", e))?;

                loop {
                    match format.next_packet() {
                        Ok(packet) => {
                            match decoder.decode(&packet) {
                                Ok(audio_buf) => {
                                    match audio_buf {
                                        AudioBufferRef::U8(_)
                                        | AudioBufferRef::U16(_)
                                        | AudioBufferRef::U24(_)
                                        | AudioBufferRef::U32(_) => {
                                            let spec = audio_buf.spec();
                                            let mut sample_buf = SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
                                            sample_buf.copy_interleaved_ref(&audio_buf);
                                            let samples = sample_buf.samples();
                                            // resample if needed
                                            let sr = spec.rate as u32;
                                            let channels = spec.channels.count();
                                            let out = crate::sfx_loader::resample_interleaved(samples, sr, crate::sfx_loader::TARGET_SAMPLE_RATE, channels as usize);
                                            // push in chunks
                                            let mut off = 0;
                                            while off < out.len() {
                                                let end = (off + 1024).min(out.len());
                                                let _ = prod.push_slice(&out[off..end]);
                                                off = end;
                                            }
                                        }
                                        AudioBufferRef::F32(buf) => {
                                            let samples = buf.as_slice();
                                            let sr = buf.spec().rate as u32;
                                            let channels = buf.spec().channels.count();
                                            let out = crate::sfx_loader::resample_interleaved(samples, sr, crate::sfx_loader::TARGET_SAMPLE_RATE, channels as usize);
                                            let mut off = 0;
                                            while off < out.len() {
                                                let end = (off + 1024).min(out.len());
                                                let _ = prod.push_slice(&out[off..end]);
                                                off = end;
                                            }
                                        }
                                    }
                                }
                                Err(_) => break,
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

        Ok(StreamingAsset { consumer: cons, _handle: handle })
    }

    pub fn read(&mut self, out: &mut [f32]) -> usize {
        self.consumer.pop_slice(out)
    }
}
