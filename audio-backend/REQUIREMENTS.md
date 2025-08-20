Audio Backend Requirements

This file enumerates the requirements for the `audio-backend` crate derived from `docs/audio_game_engine_design.md`.

1) Goals
- Provide a thin, portable audio device layer for Android (Oboe/AAudio) and Windows (CPAL/WASAPI).
- Enable <10 ms end-to-end output latency on capable Android devices and stable low latency on Windows.
- Support sample-accurate scheduling and parameter automation via frame-aligned callbacks.
- Work with the ResonanceAudioApi safe wrapper as the spatialization engine.

2) Non-goals
- The backend does not implement high-level engine features (ECS, sequencer, rooms, etc.).

3) API & Contract
- Expose trait `AudioBackend: Send` with methods:
  - `start(&mut self, render: RenderFn) -> Result<(), BackendError>`
  - `stop(&mut self)`
  - `sample_rate(&self) -> u32`
  - `buffer_size(&self) -> usize` (frames per callback)
  - `channels(&self) -> u16` (default 2)
- `RenderFn` is `Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync + 'static>`.
- Render closure semantics:
  - Called on the platform audio callback thread.
  - Fills the provided interleaved f32 buffer for `frames` frames at sample rate `sr`.

4) Real-time constraints
- Render callback must be RT-safe: no locks, no heap allocations, no logging.
- Must not panic; on error the callback should output silence.
- Pre-allocate buffers and any conversion buffers before starting the stream.

5) Platform behavior
- Android (`OboeBackend`): request PerformanceMode::LowLatency and prefer Exclusive/AAudio.
- Windows (`CpalBackend`): prefer WASAPI Exclusive, fallback to Shared.
- Expose the effective device configuration (actual sample rate, frames per callback, channels) to callers.

6) Format & conversion
- Prefer interleaved f32 device format. If the device requires i16 or planar formats, implement realtime-safe conversion with preallocated buffers.

7) Timing & clocks
- Provide an accurate device sample clock / frame count (frames since stream start) or enough info for the engine to compute frame timestamps.
- Report actual buffer sizes because platforms may force different sizes.

8) Error handling & telemetry
- Return `BackendError` variants for failures: device open, permissions, unsupported format, callback registration failures.
- Detect and report XRuns/underruns; optionally increase buffer size adaptively after repeated XRuns.
- Provide non-RT diagnostics callbacks/events for telemetry reporting.

9) Device lifecycle & hotplug
- Gracefully handle device removal and provide a recoverable error or event for the engine to re-open or switch devices.

10) Safety & threading
- All platform-specific interaction must be encapsulated; the engine interacts only with the trait and render closure.
- Avoid global locks in the callback path; communicate via pre-allocated ring buffers (engine-side) or the provided render buffer.

11) Interop with Resonance
- The backend should allow direct rendering into the device buffer (interleaved f32) so that `ResonanceCtx::render_into` can fill the output buffer directly.

12) Diagnostics & utilities
- Emit XRun telemetry, callback timing metrics, and device info (latency estimate) to a diagnostics system.
- Provide a small loopback or latency probe helper (optional) for measuring end-to-end latency.

13) Tests
- Provide unit tests for non-RT logic; a lightweight playback test harness is acceptable (see `src/test_play.rs`).
- Integration tests should check start/stop, device info reporting, and basic callback invocation (may be skipped in CI for headless environments).

14) Build & packaging
- Crate should build with `cargo build -p audio-backend`.
- For Android, provide instructions or hooks for AAR/JNI packaging and how the backend is exposed to the app layer.

15) Implementation checklist (derived)
- [ ] Define the `AudioBackend` trait, `RenderFn`, `BackendError`, and `DeviceInfo`.
- [ ] Implement `CpalAudioBackend` for Windows (WASAPI preferred).
- [ ] Implement `OboeAudioBackend` for Android.
- [ ] Expose telemetry hooks for XRuns and device changes.
- [ ] Provide realtime-safe format conversion if needed.
- [ ] Write tests and a minimal playback example.

Notes & assumptions
- Interleaved f32 is the canonical format between engine & backend.
- The engine will manage ring buffers, decoders, and higher-level scheduling; backend only owns device I/O.
- The requirements list prioritizes RT-safety and sample-accurate scheduling as primary non-functional requirements.
