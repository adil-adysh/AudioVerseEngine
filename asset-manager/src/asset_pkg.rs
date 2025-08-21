use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use memmap2::Mmap;
use crate::pkg_format::{PkgHeader, AssetIndexEntry};
use crate::util::{AssetError, PKG_MAGIC};
use sha2::{Sha256, Digest};
// Cursor not needed anymore
use bincode::{config, decode_from_slice};

pub enum MappedOrFile {
    Mmap(Mmap),
    File(File),
}

pub struct AssetPkg {
    backend: MappedOrFile,
    entries: std::collections::HashMap<String, AssetIndexEntry>,
    file_len: u64,
}

impl AssetPkg {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let file_len = file.metadata()?.len();

        let backend = if file_len > 0 {
            match unsafe { Mmap::map(&file) } {
                Ok(m) => MappedOrFile::Mmap(m),
                Err(_) => MappedOrFile::File(file),
            }
        } else {
            return Err(AssetError::InvalidPackage("empty file".into()));
        };

        // Read header serialized with bincode (use bincode for header decoding)
        let header: PkgHeader = {
            let mut header_bytes = vec![0u8; 256];
            match &backend {
                MappedOrFile::Mmap(m) => {
                    let take = &m[..std::cmp::min(m.len(), header_bytes.len())];
                    header_bytes[..take.len()].copy_from_slice(take);
                }
                MappedOrFile::File(f) => {
                    let mut f = f.try_clone()?;
                    f.seek(SeekFrom::Start(0))?;
                    f.read_exact(&mut header_bytes)?;
                }
            }
            let config = config::standard();
            match decode_from_slice::<PkgHeader, _>(&header_bytes, config) {
                Ok((h, _)) => h,
                Err(e) => return Err(AssetError::InvalidPackage(format!("header decode: {}", e))),
            }
        };

        if header.magic != PKG_MAGIC {
            return Err(AssetError::InvalidPackage("bad magic".into()));
        }

        if header.index_offset + header.index_size > file_len {
            return Err(AssetError::InvalidPackage("index OOB".into()));
        }

        // extract index bytes
        match &backend {
            MappedOrFile::Mmap(m) => {
                let start = header.index_offset as usize;
                let end = (header.index_offset + header.index_size) as usize;
                let index_bytes = &m[start..end];
                let mut hasher = Sha256::new();
                hasher.update(index_bytes);
                let h = hasher.finalize();
                if header.index_hash != h.as_slice() {
                    return Err(AssetError::InvalidPackage("index hash mismatch".into()));
                }
                let config = config::standard();
                let (entries_vec, _): (Vec<AssetIndexEntry>, usize) = decode_from_slice(index_bytes, config)
                    .map_err(|e| AssetError::InvalidPackage(format!("index decode: {}", e)))?;
                let mut map = std::collections::HashMap::new();
                for ent in entries_vec {
                    if ent.name.is_empty() || ent.name.len() > 255 {
                        return Err(AssetError::InvalidPackage("invalid asset name length".into()));
                    }
                    if ent.offset.saturating_add(ent.size) > file_len {
                        return Err(AssetError::InvalidPackage(format!("asset OOB: {}", ent.name)));
                    }
                    map.insert(ent.name.clone(), ent);
                }
                Ok(AssetPkg { backend, entries: map, file_len })
            }
            MappedOrFile::File(ref f) => {
                let mut buf = vec![0u8; header.index_size as usize];
                let mut f = f.try_clone()?;
                f.seek(SeekFrom::Start(header.index_offset))?;
                f.read_exact(&mut buf)?;
                let mut hasher = Sha256::new();
                hasher.update(&buf);
                let h = hasher.finalize();
                if header.index_hash != h.as_slice() {
                    return Err(AssetError::InvalidPackage("index hash mismatch".into()));
                }
                let config = config::standard();
                let (entries_vec, _): (Vec<AssetIndexEntry>, usize) = decode_from_slice(&buf, config)
                    .map_err(|e| AssetError::InvalidPackage(format!("index decode: {}", e)))?;
                let mut map = std::collections::HashMap::new();
                for ent in entries_vec {
                    if ent.name.is_empty() || ent.name.len() > 255 {
                        return Err(AssetError::InvalidPackage("invalid asset name length".into()));
                    }
                    if ent.offset.saturating_add(ent.size) > file_len {
                        return Err(AssetError::InvalidPackage(format!("asset OOB: {}", ent.name)));
                    }
                    map.insert(ent.name.clone(), ent);
                }
                Ok(AssetPkg { backend, entries: map, file_len })
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&AssetIndexEntry> {
        self.entries.get(name)
    }

    pub fn read_asset_bytes(&self, name: &str) -> Result<Vec<u8>, AssetError> {
        let ent = self.entries.get(name).ok_or(AssetError::NotFound)?;
        if ent.size > (1u64 << 31) {
            return Err(AssetError::ResourceLimit("asset too large".into()));
        }
        match &self.backend {
            MappedOrFile::Mmap(m) => {
                let start = ent.offset as usize;
                let end = (ent.offset + ent.size) as usize;
                Ok(m[start..end].to_vec())
            },
            MappedOrFile::File(ref f) => {
                let mut f = f.try_clone()?;
                let mut buf = vec![0u8; ent.size as usize];
                f.seek(SeekFrom::Start(ent.offset))?;
                f.read_exact(&mut buf)?;
                Ok(buf)
            }
        }
    }

    pub fn file_len(&self) -> u64 { self.file_len }
}

