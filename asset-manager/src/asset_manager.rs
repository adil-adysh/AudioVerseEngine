use std::collections::HashMap;
use std::path::Path;
// Arc is used inside feature-gated streaming module; keep direct imports local there.

use crate::sfx_loader;
pub use crate::sfx_loader::SfxMetadata;
// streaming_loader is only referenced when the streaming feature is enabled.
// Re-export StreamingAsset when the feature is enabled (placed before tests
// to satisfy clippy's `items_after_test_module` lint).
#[cfg(feature = "streaming")]
pub use crate::streaming_loader::StreamingAsset;

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

#[derive(Default)]
pub struct AssetManager {
    // simple name -> path mapping for this skeleton
    assets: HashMap<String, String>,
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
    sfx_loader::load_sfx_path_with_target(p, sfx_loader::TARGET_SAMPLE_RATE)
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


