Audio Backend Requirements (Rust-focused, actionable)

This file lists the concrete requirements for the `audio-backend` crate, prioritized for an audio game engine. It also contains an implementation plan and constraints to guide the refactor.

Goals
- Provide a thin, portable audio device layer for Windows (CPAL/WASAPI) and Android (Oboe/AAudio) that prioritizes RT-safety and sample-accurate scheduling.
- Deliver a Rust-native, testable API that the engine can use to perform sample-accurate rendering directly into an interleaved f32 device buffer.

Non-goals
- No high-level engine features (sequencers, ECS) — backend only handles device I/O, format conversion, and diagnostics.

API & Contract (must-have)
- Trait: `AudioBackend: Send` (engine expects backend handles to be Send).
  - Methods:
    - `start(&mut self, render: RenderFn) -> Result<(), BackendError>`
    - `stop(&mut self) -> Result<(), BackendError>`
    - `sample_rate(&self) -> u32`
    - `buffer_size(&self) -> usize` (frames per callback)
    - `channels(&self) -> u16` (default 2)
    - `frames_since_start(&self) -> u64`
    - `set_diagnostics_callback(&mut self, cb: Option<DiagnosticsCb>)`
- `RenderFn` type: `Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync + 'static>`.
  - Called on the platform audio callback thread to fill the interleaved f32 buffer for `frames` frames at sample rate `sr`.

Real-time constraints (non-negotiable)
- No heap allocations, no locks (no Mutex) in the audio callback. Use lock-free techniques (e.g., `arc-swap`) for render closure lookup.
- No logging from the audio callback.
- Must not panic; on error, output silence and surface non-RT diagnostics.
- Pre-allocate conversion buffers and re-use them in the callback.

Platform behavior (implementation notes)
- Windows: prefer WASAPI exclusive mode when available; fall back to shared.
- Android: prefer AAudio/Oboe low-latency path when available.
- Expose the effective device configuration (sample rate, frames per callback, channels) via `DeviceInfo`.

Formats & conversion
- Canonical internal format: interleaved f32.
- If device uses a different format (i16 or planar), worker thread must do realtime-safe conversion using preallocated buffers.

Timing & clocks
- Provide frames-since-start counter (atomic u64) and expose it via `frames_since_start()`.
- Provide enough data for engine to compute frame timestamps (sample rate + frames counter).

Diagnostics & telemetry
- Emit `DiagnosticEvent` for XRuns, device removal, and buffer-size changes via a non-RT `DiagnosticsCb`.
- XRuns detected via CPAL error callback should increment an atomic counter and trigger a non-RT callback.

Device lifecycle & hotplug
- Worker thread should surface device removal events and allow the engine to recreate or switch devices.

Tests & examples
- Unit tests for non-RT logic.
- Example `examples/play_sine.rs` demonstrating creating backend and rendering a tone (skipped in CI on headless runners).

Implementation plan (phase 1 — prioritized)
1. Add `arc-swap` dependency for lock-free render closure lookup.
2. Refactor `CpalAudioBackend` into a Send-safe handle that communicates with a worker thread:
   - The handle is Send and contains control tx, DeviceInfo, and atomics.
   - The worker thread owns the CPAL `Stream` and preallocated buffers and performs RT-safe callbacks.
3. Replace Mutex-based RT access with `ArcSwapOption<RenderFn>` read in the callback.
4. Implement realtime-safe conversion buffers (i16 -> f32, planar -> interleaved) in worker thread.
5. Add diagnostics event emission and `frames_since_start` atomic counter.
6. Update `mock_backend` to the same handle pattern for parity.
7. Add minimal unit tests and an example playback harness.

Notes & trade-offs
- Keeping `AudioBackend: Send` requires a Send-safe public handle. The worker thread will own any non-Send platform objects. This avoids unsafe Send impls and keeps the trait contract.
- Some platform-specific low-level tuning (WASAPI exclusive flags, AAudio performance mode) is left for phase 2.

Acceptance criteria for phase 1
- `cargo build -p audio-backend` passes.
- `create_audio_backend()` returns a Send handle that can start a stream and accept a `RenderFn` with no locks in the RT path.
- Diagnostics events are emitted for errors/XRuns.

