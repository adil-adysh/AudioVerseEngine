Audio-first game engine — must-have features
============================================

This document lists the minimal, practical feature set required for an audio-first game engine (what "must" exist for the engine to be usable as an audio platform). The list is prioritized and includes short acceptance criteria and low-risk next steps.

1) Deterministic, sample-accurate audio render loop
   - Acceptance: Engine exposes a render callback that processes exactly N frames per callback at a fixed sample rate (48 kHz) and supports interleaved and planar buffers.
   - Why: All DSP and spatialization algorithms need deterministic timing and exact sample counts.
   - Next step: Verify `audio-backend` provides a reliable callback and a small smoke test that writes known samples and validates output length.

2) Asset pipeline + canonical `.sfx` format (48 kHz, f32 interleaved)
   - Acceptance: Tools can convert .wav/.ogg/.opus into `.sfx`; `asset-manager` can load `.sfx` into memory as canonical `SfxBlob` with correct header parsing.
   - Why: Single canonical format simplifies streaming, resampling, and memory layouts for audio DSP.
   - Next step: Add a unit test that covers `tools::asset-utils::convert_to_sfx_bytes` and `asset-manager::SfxBlob::from_sfx_bytes` roundtrip.

3) Spatial audio API + room geometry
   - Acceptance: Public API for creating sources, listener, and room/geometry; room properties (reverb, occlusion) must be queryable and updatable at runtime.
   - Why: Spatialization and room effects are core to audio-first games.
   - Next step: Add minimal runtime API in `engine-core` to register sources and a room description, with integration tests against `resonance-audio` bridge.

4) Low-latency streaming and voice support
   - Acceptance: Engine supports streaming large files and low-latency voice sources; supports gapless playback and prioritized voice channels.
   - Next step: Add streaming loader unit tests and a mock backend test that exercises begin/stop/pause with no full-file preloading.

5) Thread-safe command posting / control surface
   - Acceptance: Systems can post commands to the audio thread (start/stop/set-param) using lockless queues or message passing; posted commands are executed in the audio render callback.
   - Next step: Validate `resonance-audio` task queue patterns (see `lockless_task_queue.h`) and add a Rust wrapper that exposes a safe command API.

6) Deterministic DSP primitives & offline testing
   - Acceptance: DSP building blocks (sample-rate conversion, filters, panners) have deterministic reference tests; offline rendering of frames possible for automated testing.
   - Next step: Add a small offline render test that processes a known input buffer and compares CRC or sample values.

7) Simple, deterministic API for moving listener and sources
   - Acceptance: An API for updating position, velocity, orientation per-frame exists; velocity-based Doppler is computed from sampled positions.
   - Next step: Verify `update_player_state_system` and `doppler_effect_system` provide the required data to the audio system.

8) CI checks that compile/respect native bridge (resonance-cxx)
   - Acceptance: CI builds the C++ resonance bridge and runs `cargo build` to validate cxx bindings are healthy.
   - Next step: Add a CI job that runs CMake for `resonance-audio` or uses placeholder implementations to link the cxx bridge.

9) Simple profiling & debug visualizations for audio paths
   - Acceptance: Developer-facing hooks to sample levels, log processing time per frame, and optionally render debug visuals for audio geometry.
   - Next step: Add a lightweight `audio-diagnostics` plugin that subscribes to audio events and writes basic metrics to logs.

Low-risk immediate actions to take now
- Add `docs/audio-must-haves.md` (this file) and link it from `README.md`.
- Add unit tests for `.sfx` roundtrip and `PrevPlayerPos`/velocity computation.
- Add a small CI snippet to build the cxx bridge using a minimal placeholder implementation.

If you want, I can implement any of the next-step items now. Tell me which one to tackle first.
