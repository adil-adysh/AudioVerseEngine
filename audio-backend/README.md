audio-backend crate

Purpose

The `audio-backend` crate provides the platform audio device abstraction for the AudioVerseEngine. It opens and manages the low-level audio stream and calls into the engine's realtime render closure. Implementations live under platform-specific modules (e.g. `CpalAudioBackend` for Windows, `OboeAudioBackend` for Android).

Design contract

- Trait: `AudioBackend`
  - `start(&mut self, render: RenderFn) -> Result<(), BackendError>`: starts the audio stream and registers the realtime render callback.
  - `stop(&mut self)`: stops the stream and cleans up resources.
  - `sample_rate(&self) -> u32`: returns the effective sample rate.
  - `buffer_size(&self) -> usize`: frames per callback.
  - `channels(&self) -> u16`: channel count (default 2).
- `RenderFn`: `Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync + 'static>` — called on the audio callback thread.

Realtime rules

- The render closure runs on the audio callback thread and must be realtime-safe:
  - No locks, no heap allocations, no logging.
  - Must never panic; on error it should write silence to the output buffer.

Platform implementations

- `CpalAudioBackend` (Windows): prefer WASAPI Exclusive, fallback to shared.
- `OboeAudioBackend` (Android): prefer AAudio/PerformanceMode::LowLatency and exclusive where available.

Usage (example)

- The crate contains a simple test utility in `src/test_play.rs` showing how to create a backend and play a stereo buffer. That file demonstrates the API shape and can be used as a starting point when wiring the engine.

Building & testing

- Build the crate with the workspace root Cargo invocation. Example:

```powershell
# from repo root
cargo build -p audio-backend
cargo test -p audio-backend
```

Notes & diagnostics

- The backend should emit telemetry for XRuns/underruns and expose device info (sample rate, buffer size, channels) to the engine.
- Prefer interleaved f32 for render buffers. If the platform/device requires a different sample format, implement realtime-safe conversion with a preallocated buffer.

Where to extend

- Implement platform backends under `src/` and export them from `lib.rs`.
- Add a `BackendError` type and a `DeviceInfo` struct for richer error and capability reporting.

License

- Follow the repository license.
