//! Minimal Asset Manager API surface.
pub mod asset_manager;
pub mod asset_pkg;
pub mod loader;
pub mod pkg_format;
pub mod sfx;
pub mod sfx_loader;
pub mod streaming_loader;
pub mod util;

pub use asset_manager::{AssetManager, Error};
pub use util::AssetError;

#[cfg(feature = "streaming")]
pub use streaming_loader::StreamingAsset;
