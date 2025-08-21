use std::sync::{Arc, Mutex};
use lru::LruCache;
// shorten complex cache type for clarity and to satisfy clippy's type_complexity lint
type SfxCache = LruCache<String, (usize, Arc<crate::sfx::SfxBlob>)>;
use crate::asset_pkg::AssetPkg;
use crate::sfx::SfxBlob;
use crate::util::{AssetError, DEFAULT_SFX_MEMORY_BUDGET_BYTES};
use std::thread;

#[derive(Clone)]
pub struct AssetLoader {
    pkg: Option<std::sync::Arc<AssetPkg>>,
    cache: Arc<Mutex<SfxCache>>,
    memory_budget: usize,
}

impl AssetLoader {
    pub fn from_pkg(path: impl AsRef<std::path::Path>, memory_budget: usize) -> Result<Self, AssetError> {
    let pr = AssetPkg::open(path)?;
    let pr = std::sync::Arc::new(pr);
        let cache = LruCache::unbounded();
        Ok(AssetLoader {
            pkg: Some(pr),
            cache: Arc::new(Mutex::new(cache)),
            memory_budget,
        })
    }

    pub fn from_pkg_default(path: impl AsRef<std::path::Path>) -> Result<Self, AssetError> {
        Self::from_pkg(path, DEFAULT_SFX_MEMORY_BUDGET_BYTES)
    }

    pub fn load_sfx_sync(&self, name: &str) -> Result<Arc<SfxBlob>, AssetError> {
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some((_, blob)) = cache.get(name) {
                return Ok(blob.clone());
            }
        }

            let pr = self.pkg.as_ref().ok_or(AssetError::InvalidPackage("no package".into()))?;
        let bytes = pr.read_asset_bytes(name)?;
        let blob = SfxBlob::from_sfx_bytes(&bytes)?;
        let size_bytes = blob.samples.len() * 4;
        self.insert_or_evict(name.to_string(), size_bytes, Arc::new(blob.clone()))?;

        let mut cache = self.cache.lock().unwrap();
        let (_, blob_arc) = cache.get(name).unwrap().clone();
        Ok(blob_arc)
    }

    pub fn prefetch(&self, name: &str) {
        let self_cloned = self.clone();
        let name_owned = name.to_string();
        thread::spawn(move || {
            let _ = self_cloned.load_sfx_sync(&name_owned);
        });
    }

    fn insert_or_evict(&self, key: String, size_bytes: usize, blob: Arc<SfxBlob>) -> Result<(), AssetError> {
        let mut cache = self.cache.lock().unwrap();
        cache.put(key.clone(), (size_bytes, blob));
        let mut total = 0usize;
        for (_k, (sz, _)) in cache.iter() {
            total += *sz;
        }
        while total > self.memory_budget {
            if let Some((_k, (sz, _))) = cache.pop_lru() {
                total = total.saturating_sub(sz);
            } else {
                break;
            }
        }
        Ok(())
    }
}
