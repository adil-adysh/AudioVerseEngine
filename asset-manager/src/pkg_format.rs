use bincode::{Encode, Decode};
use sha2::{Sha256, Digest};
use crate::util::{PKG_MAGIC, PKG_VERSION};

#[derive(Encode, Decode, Debug, Clone)]
pub struct PkgHeader {
    pub magic: u32,
    pub version: u16,
    pub flags: u16,
    pub index_offset: u64,
    pub index_size: u64,
    pub index_hash: [u8; 32],
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum AssetType {
    Sfx = 0,
    Music = 1,
    Other = 2,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct AssetIndexEntry {
    pub name: String,
    pub asset_type: AssetType,
    pub offset: u64,
    pub size: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub flags: u16,
    pub checksum: Option<[u8; 32]>,
}

impl PkgHeader {
    pub fn new(index_offset: u64, index_bytes: &[u8], flags: u16) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(index_bytes);
        let hash = hasher.finalize();
        let mut h = [0u8; 32];
        h.copy_from_slice(&hash);
        PkgHeader {
            magic: PKG_MAGIC,
            version: PKG_VERSION,
            flags,
            index_offset,
            index_size: index_bytes.len() as u64,
            index_hash: h,
        }
    }
}
