# Audio Game Engine TODOs

## Easy/Foundational Tasks
1. Set up Rust workspace and crate structure (`audio-backend`, `resonance-ffi`, `resonance`, `engine-core`, `content-pipeline`, `app-android`, `app-windows`).
2. Implement `resonance-ffi`: raw externs for ResonanceAudioApi FFI surface.
3. Create safe wrapper `ResonanceCtx` for FFI in Rust.
4. Implement `audio-backend` trait and basic CPAL (Windows) backend.
5. Implement lock-free command queue and frame scheduler (single-producer, single-consumer).

6. Implement basic source registry and ring buffer logic.
7. Implement minimal Engine API: init, shutdown, create/destroy source, play clip, set listener pose.
8. Implement simple ECS setup with `hecs` (entities, components for audio sources/listener).
9. Implement basic content loader for RON/JSON assets.

## Intermediate Tasks
10. Implement Oboe (Android) backend and JNI wrapper.
11. Integrate ResonanceAudioApi for spatialization and room effects.
12. Implement room, occlusion, and sequencer ECS systems.
13. Implement streaming for long audio files (lock-free ring buffer per source).
14. Implement asset validation (sample rate, channel layout, loudness scan).
15. Implement telemetry: XRun detection, output meter, latency probes.
16. Implement dynamic range profiles and ducking logic.

## Advanced/Polish Tasks
17. Implement cross-platform build scripts (Windows launcher, Android AAR packaging).
18. Implement DOT dump of ECS and audio routing for diagnostics.
19. Implement adaptive buffer sizing and voice capping.
20. Implement limiter (optional) before render.
21. Implement loopback test harness for latency measurement.
22. Finalize documentation and minimal example code.

## Testing
23. Write unit tests for command ordering, scheduler, ring buffer, parameter smoothing.
24. Write DSP tests (null rendering, denormals, buffer wrap correctness).
25. Write performance tests (128+ sources, no XRuns).
26. Write room/transition tests (crossfade, property updates).

---

*Start with foundational tasks and progress to intermediate and advanced features. Prioritize easy-to-implement, core engine features first.*
