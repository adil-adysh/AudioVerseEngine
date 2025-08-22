# asset-manager (mini README)

This crate provides a small asset packaging and loading utility for the engine.

Key types
- `asset_manager::asset_pkg::AssetPkg` — low-level package reader for `.pkg` files.
- `asset_manager::AssetManager` / `asset_manager::AssetLoader` — higher-level helpers for registering and loading assets.

Convenience helpers added

AssetPkg
- `AssetPkg::open(path)` — open a `.pkg` file (mmap when possible).
- `pkg.get(name) -> Option<&AssetIndexEntry>` — lookup metadata for an asset by name.
- `pkg.read_asset_bytes(name) -> Result<Vec<u8>, AssetError>` — read raw bytes of an asset.
- `pkg.list_names() -> Vec<String>` — sorted list of asset names in the package.
- `pkg.entries_vec() -> Vec<AssetIndexEntry>` — cloned vector of index entries.
- `pkg.read_sfx_blob(name) -> Result<SfxBlob, AssetError>` — parse an `.sfx` asset directly to `SfxBlob`.
- `pkg.read_sfx_samples(name) -> Result<(Vec<f32>, SfxMetadata), AssetError>` — parse an asset into interleaved samples and metadata.

AssetLoader
- `AssetLoader::from_pkg(path, budget)` / `from_pkg_default(path)` — create a loader backed by a package.
- `loader.list_names() -> Option<Vec<String>>` — list names when loader has a package.
- `loader.read_asset_raw(name) -> Result<Vec<u8>, AssetError>` — read raw bytes pulled from the package.
- `loader.load_sfx_sync(name) -> Result<Arc<SfxBlob>, AssetError>` — cached SFX loading (existing).

Usage examples

Open a package and read an SFX asset:

```rust
use asset_manager::asset_pkg::AssetPkg;

let pkg = AssetPkg::open("assets/dest/out.pkg")?;
let bytes = pkg.read_asset_bytes("assets/sfx/card-fan-1.sfx")?;
let blob = pkg.read_sfx_blob("assets/sfx/card-fan-1.sfx")?;

// or samples + metadata
let (samples, meta) = pkg.read_sfx_samples("assets/sfx/card-fan-1.sfx")?;
```

Create an AssetLoader backed by the package and list assets:

```rust
use asset_manager::AssetLoader;
let loader = AssetLoader::from_pkg_default("assets/dest/out.pkg")?;
if let Some(names) = loader.list_names() {
    for n in names { println!("{}", n); }
}
let blob = loader.load_sfx_sync("assets/sfx/card-fan-1.sfx")?;
```

Notes
- Asset names are stored and looked up as exact strings (case-sensitive).
- `read_asset_bytes` returns owned bytes (copies when using mmap backend).
- `SfxBlob` provides parsed fields and interleaved sample data.

If you'd like different helper shapes (zero-copy iterators, streaming readers, JSON listing output), tell me which and I will add them.
