Assets layout and packing
=========================

Project expects an `assets/` folder at the repository root with the following structure:

- assets/
  - sfx/        # Small predecoded sounds (source .wav/.ogg/.opus or prebuilt .sfx)
  - audio/      # Music/streamed files that should remain compressed (ogg/mp3/wav)
  - dest/       # Output location where `asset-packer --pack-assets` writes the final `out.pkg`

Workflow
--------
1. Put small sound effects (wav/ogg/opus) into `assets/sfx/`. They will be converted to the engine `.sfx` pre-decoded format during packing.
2. Put larger music/dialogue files into `assets/audio/`. These are packed without conversion and streamed at runtime.
3. From the repository root run:

```powershell
# Windows PowerShell
.\tools\pack-assets.ps1
```

Or run directly with cargo:

```powershell
cargo run -p asset-packer -- --pack-assets
```

This will create `assets/dest/out.pkg` containing packed assets and an index.

Manual conversion (optional)
----------------------------
To convert a single file to `.sfx` manually (for inspection or to customize names):

```powershell
cargo run -p sfx-convert -- out.sfx path\to\in.wav
```

Implementation notes
--------------------
- Converted `.sfx` files use the `SFX1` binary layout (f32 interleaved, little-endian, 48kHz target).
- The packer uses `symphonia` to probe metadata and `rubato` for high-quality resampling.
- If you want a different behavior (size limit for conversion, output name), I can add CLI flags to `asset-packer`.
