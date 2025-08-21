//! Minimal Asset Manager API surface.
pub mod asset_manager;
pub mod asset_pkg;
pub mod sfx_loader;
pub mod streaming_loader;

pub use asset_manager::{AssetManager, Error};

#[cfg(feature = "streaming")]
pub use streaming_loader::StreamingAsset;
