sfx-convert
===========

Small CLI to convert common audio files (wav/ogg/mp3) to the project's `.sfx` pre-decoded format.

Usage:

```
# build
cargo build -p sfx-convert

# convert a file
cargo run -p sfx-convert -- out.sfx in.wav
```

Notes:
- The tool decodes audio using `symphonia` and resamples to 48 kHz (project target) using `rubato`.
- Output format is the project's `SFX1` binary layout (sample format = f32, interleaved).
- The resulting `.sfx` can be passed into `tools/asset-packer` to produce final `asset.pkg` files.
