use std::collections::HashMap;
use std::path::Path;
// Arc is used inside feature-gated streaming module; keep direct imports local there.

use crate::sfx_loader;
pub use crate::sfx_loader::SfxMetadata;
#[cfg(feature = "streaming")]
use crate::streaming_loader;

#[derive(Debug)]
pub enum Error {
    NotFound,
    Io(std::io::Error),
    Decode(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

// SfxMetadata is provided by `sfx_loader::SfxMetadata` and re-exported above.

pub struct AssetManager {
    // simple name -> path mapping for this skeleton
    assets: HashMap<String, String>,
}

impl Default for AssetManager {
    fn default() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn register_asset(&mut self, name: impl Into<String>, path: impl Into<String>) {
        self.assets.insert(name.into(), path.into());
    }

    /// Load a pre-decoded SFX as interleaved f32 PCM samples.
    pub fn load_sfx(&self, name: &str) -> Result<(Vec<f32>, SfxMetadata), Error> {
        let path = self.assets.get(name).ok_or(Error::NotFound)?;
        let p = Path::new(path);
        sfx_loader::load_sfx_path(p)
    }

    /// Load a streaming asset; feature-gated stub that returns a StreamingAsset handle.
    #[cfg(feature = "streaming")]
    pub fn load_stream(&self, name: &str) -> Result<StreamingAsset, Error> {
        let path = self.assets.get(name).ok_or(Error::NotFound)?;
        StreamingAsset::open(path).map_err(|e| Error::Decode(format!("stream open: {}", e)))
    }

    #[cfg(not(feature = "streaming"))]
    pub fn load_stream(&self, _name: &str) -> Result<(), Error> {
        Err(Error::Decode("streaming feature not enabled".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sfx_load_stub() {
        let mut mgr = AssetManager::new();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // create minimal fake sfx: channels=2, sample_rate=48000, one sample 0.0
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&48000u32.to_le_bytes());
        buf.extend_from_slice(&0f32.to_le_bytes());
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&buf).unwrap();
            f.sync_all().unwrap();
        }

        mgr.register_asset("beep", path.to_str().unwrap());
        let (samples, meta) = mgr.load_sfx("beep").unwrap();
        assert_eq!(meta.channels, 2);
        assert_eq!(meta.sample_rate, 48000);

        assert_eq!(samples.len(), 1);
    }
}

// Streaming types
#[cfg(feature = "streaming")]
pub mod streaming {
    use ringbuf::{
        traits::{Consumer, Producer},
        HeapRb,
    };
    use std::path::Path;
    use std::thread;

    /// Handle representing a streaming asset. Consumer is returned to the caller
    /// to read interleaved f32 samples. A background thread decodes and pushes
    /// samples into the producer side.
    pub struct StreamingAsset {
        consumer: ringbuf::HeapCons<f32>,
        // Keep the decoder thread handle so it lives as long as the asset.
        _handle: thread::JoinHandle<()>,
    }

    impl StreamingAsset {
        pub fn open(path: &str) -> Result<StreamingAsset, String> {
            // Create a reasonably sized ring buffer for streaming audio
            let rb = HeapRb::<f32>::new(32 * 1024);
            let (mut prod, cons) = rb.split();

            let path_str = path.to_string();
            let handle = thread::spawn(move || {
                // Decoder thread: try to use symphonia; fall back to raw pcm if extension is .pcm
                let p = Path::new(&path_str);
                match p
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                {
                    Some(ext) if ext == "pcm" || ext == "sfx" => {
                        // read raw f32 LE samples after an 8-byte header (our simple format)
                        if let Ok(data) = std::fs::read(&path_str) {
                            if data.len() > 8 {
                                let mut i = 8usize;
                                let mut tmp = Vec::with_capacity((data.len() - 8) / 4);
                                while i + 4 <= data.len() {
                                    let b = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                                    tmp.push(f32::from_le_bytes(b));
                                    i += 4;
                                }
                                // push in chunks
                                let mut offset = 0;
                                while offset < tmp.len() {
                                    let end = (offset + 1024).min(tmp.len());
                                    // ignore full/partial pushes for brevity
                                    let _ = prod.push_slice(&tmp[offset..end]);
                                    offset = end;
                                }
                            }
                        }
                    }
                    _ => {
                        // Try symphonia decoding. If it fails, exit thread.
                        if let Err(e) = Self::decode_with_symphonia(&path_str, &mut prod) {
                            eprintln!("stream decode error: {}", e);
                        }
                    }
                }
            });

            Ok(StreamingAsset {
                consumer: cons,
                _handle: handle,
            })
        }

        fn decode_with_symphonia(path: &str, prod: &mut impl Producer<f32>) -> Result<(), String> {
            use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
            use symphonia::core::codecs::DecoderOptions;
            use symphonia::core::formats::FormatOptions;
            use symphonia::core::io::MediaSourceStream;
            use symphonia::core::meta::MetadataOptions;
            use symphonia::default::{get_codecs, get_probe};

            let file = std::fs::File::open(path).map_err(|e| format!("open: {}", e))?;
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
                                        // convert using SampleBuffer
                                        let spec = audio_buf.spec();
                                        let mut sample_buf = SampleBuffer::<f32>::new(
                                            audio_buf.capacity() as u64,
                                            spec,
                                        );
                                        sample_buf.copy_interleaved_ref(&audio_buf);
                                        let samples = sample_buf.samples();
                                        let mut offset = 0;
                                        while offset < samples.len() {
                                            let end = (offset + 1024).min(samples.len());
                                            let _ = prod.push_slice(&samples[offset..end]);
                                            offset = end;
                                        }
                                    }
                                    AudioBufferRef::F32(buf) => {
                                        let samples = buf.as_slice();
                                        let mut offset = 0;
                                        while offset < samples.len() {
                                            let end = (offset + 1024).min(samples.len());
                                            let _ = prod.push_slice(&samples[offset..end]);
                                            offset = end;
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
        }

        /// Read up to `out.len()` interleaved f32 samples from the consumer.
        pub fn read(&mut self, out: &mut [f32]) -> usize {
            self.consumer.pop_slice(out)
        }
    }
}

#[cfg(feature = "streaming")]
pub use streaming::StreamingAsset;
