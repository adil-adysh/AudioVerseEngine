asset-packer

A small, easy-to-use CLI for creating the repository .pkg asset bundles consumed by the `asset-manager` crate.

Features
- Recursively discover files when you pass directories.
- Compute a per-asset SHA-256 checksum and store it in the index.
- Probe common audio formats (wav, ogg, mp3, etc.) using `symphonia` and fill sample rate + channel count when available.
- Writes a bincode-serialized index and a header (with SHA-256 of the index) so `AssetPkg::open` can validate packages.

Quick usage

Build and run via cargo from the workspace root:

```sh
cargo run -p asset-packer -- out.pkg path/to/dir another/file.sfx
```

Examples

- Pack a directory (recursively):

```sh
cargo run -p asset-packer -- -r assets/ assets.pkg
```

- Pack a few files directly:

```sh
cargo run -p asset-packer -- out.pkg assets/menu.sfx assets/music/theme.ogg
```

Recursive behavior

- Use `-r` or `--recursive` before the output filename to scan directories recursively.
- Without `-r`, when you pass a directory only the immediate files (non-recursive) in that directory are packed.

Examples:

```sh
# recursive: scans subdirectories
cargo run -p asset-packer -- -r out.pkg assets/

# non-recursive: only top-level files in `assets/` are considered
cargo run -p asset-packer -- out.pkg assets/
```

How it works (short)
- The packer writes a fixed-size header placeholder, then appends the raw bytes of each input file in order.
- It collects an index of `AssetIndexEntry` records (name, type, offset, size, sample_rate, channels, flags, checksum).
- The index is serialized with `bincode` and appended after the assets. The header is then written with the index offset, size, and an SHA-256 of the index bytes so the reader can verify integrity.

Notes & limitations
- Asset names are the path strings of files passed to the packer (use relative paths for nicer names).
- The packer probes audio metadata with `symphonia` and will fill sample rate and channel count when it can; it does not transcode or resample files.
- The header placeholder is currently 512 bytes. If a future header grows beyond that size the packer will return an error — we can extend the format if needed.
- There is no manifest file format yet; if you want predictable asset names or custom metadata, we can add JSON/TOML manifest support.

Try it

1. From workspace root, build quickly:

```sh
cargo build -p asset-packer --release
```

2. Run the packer (example):

```sh
./target/release/asset-packer out.pkg assets/
```

Future improvements
- Add a manifest mode (JSON/TOML) to control asset names and metadata.
- Add compression option for asset payloads.
- Provide a "dry-run" mode that prints what would be packed without writing a file.

License / attribution
- This tool is a helper for the AudioVerseEngine repository and follows the same license as the project.
