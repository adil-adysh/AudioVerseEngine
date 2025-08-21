use thiserror::Error;

/// Constants & small helpers
pub const PKG_MAGIC: u32 = 0x41564750; // 'PVGA' chosen magic
pub const PKG_VERSION: u16 = 1;

// safety caps
pub const MAX_SFX_FRAMES: u64 = 100_000_000; // sanity cap (100M frames)
pub const DEFAULT_ENGINE_SR: u32 = 48_000;
pub const DEFAULT_SFX_MEMORY_BUDGET_BYTES: usize = 64 * 1024 * 1024; // 64 MB

#[derive(Error, Debug)]
pub enum AssetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid package: {0}")]
    InvalidPackage(String),
    #[error("asset not found")]
    NotFound,
    #[error("decode error: {0}")]
    Decode(String),
    #[error("resource limits exceeded: {0}")]
    ResourceLimit(String),
    #[error("streaming feature not enabled")]
    StreamingFeatureDisabled,
}
