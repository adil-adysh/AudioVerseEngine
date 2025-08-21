# Resonance Audio: Logical Rust Crates Structure

This document describes a recommended logical crate split for the `resonance-audio` engine, extracted from the C++ headers and architecture in `resonance-audio`. The goal is to separate concerns, make dependencies explicit, and give a clear mapping from the C++ headers to crate boundaries.

The suggested crates are designed for a Rust port or a Rust/C++ hybrid workspace using `cxx` or `bindgen`. Each crate lists purpose, key headers (from the original C++ repo), dependencies, and short notes about what to port or keep in C++.

---

## 1. resonance-audio-base
Purpose: Core, header-only primitives and utilities used across the codebase. No internal crate dependencies.

Key headers (C++):
- `resonance-audio/resonance_audio/base/audio_buffer.h`
- `resonance-audio/resonance_audio/base/constants_and_types.h`
- `resonance-audio/resonance_audio/utils/lockless_task_queue.h`
- `resonance-audio/resonance_audio/base/aligned_allocator.h`
- `resonance-audio/resonance_audio/base/object_transform.h`
- `resonance-audio/resonance_audio/base/misc_math.h`

Notes:
- This crate should expose safe, ergonomic Rust equivalents for multi-channel planar audio buffers and Channel views.
- The lock-free task queue can be implemented with Rust atomics and crossbeam-like primitives (or reused via C++ if desired).
- Keep SIMD utilities behind a nightly/feature gated module or implement with portable Rust simd crates.

---

## 2. resonance-audio-dsp
Purpose: DSP primitives and effects; depends on `resonance-audio-base`.

Key headers (C++):
- `resonance-audio/resonance_audio/dsp/fft_manager.h`
- `resonance-audio/resonance_audio/dsp/partitioned_fft_filter.h`
- `resonance-audio/resonance_audio/dsp/spectral_reverb.h`
- `resonance-audio/resonance_audio/dsp/stereo_panner.h`
- `resonance-audio/resonance_audio/dsp/distance_attenuation.h`
- `resonance-audio/resonance_audio/dsp/mono_pole_filter.h`

Dependencies: resonance-audio-base

Notes:
- FFT and partitioned convolution are core; consider using rustfft (or FFTW via FFI) for an initial Rust implementation.
- Provide clear API boundaries: small, well-typed structs for filters and process calls that accept `AudioBuffer` types from `resonance-audio-base`.

---

## 3. resonance-audio-ambisonics
Purpose: Ambisonic encoding/decoding and HRTF/SH utilities; depends on `base` and `dsp`.

Key headers (C++):
- `resonance-audio/resonance_audio/ambisonics/ambisonic_binaural_decoder.h`
- `resonance-audio/resonance_audio/ambisonics/hoa_rotator.h`
- `resonance-audio/resonance_audio/ambisonics/ambisonic_codec.h`
- `resonance-audio/resonance_audio/ambisonics/ambisonic_lookup_table.h`

Dependencies: resonance-audio-base, resonance-audio-dsp

Notes:
- This crate consumes Ambisonic SH coefficients and emits stereo/binaural outputs.
- Implement spherical harmonic helpers and provide a clear path for HRTF asset loading.

---

## 4. resonance-audio-acoustics
Purpose: Geometrical acoustics, ray tracing and IR computation; depends on `base` and an external ray-tracing dependency (Embree or a Rust equivalent).

Key headers (C++):
- `resonance-audio/resonance_audio/geometrical_acoustics/scene_manager.h`
- `resonance-audio/resonance_audio/geometrical_acoustics/path_tracer.h`
- `resonance-audio/resonance_audio/geometrical_acoustics/impulse_response_computer.h`
- `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_source.h`
- `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_listener.h`

Dependencies: resonance-audio-base (+ Embree or similar)

Notes:
- Consider keeping heavy Embree FFI in C++ and expose a safe Rust API; or port path tracer to a Rust-based ray tracer if performance/portability demands it.
- Provide high-level helpers for computing room RT60 and reflection kernels that can be called asynchronously.

---

## 5. resonance-audio-graph
Purpose: The audio node graph, source lifecycle and processing orchestration; depends on base, dsp, ambisonics, acoustics.

Key headers (C++):
- `resonance-audio/resonance_audio/graph/graph_manager.h`
- `resonance-audio/resonance_audio/graph/source_parameters_manager.h`
- `resonance-audio/resonance_audio/node/node.h`
- `resonance-audio/resonance_audio/graph/reverb_node.h`
- `resonance-audio/resonance_audio/graph/reflections_node.h`
- `resonance-audio/resonance_audio/graph/occlusion_node.h`
- `resonance-audio/resonance_audio/graph/near_field_effect_node.h`
- `resonance-audio/resonance_audio/graph/buffered_source_node.h`

Dependencies: resonance-audio-base, resonance-audio-dsp, resonance-audio-ambisonics, resonance-audio-acoustics

Notes:
- This crate is the core runtime executed by the engine: it should accept per-source parameter updates (ideally via a lockless queue) and expose a `process()` function that fills output buffers.
- Keep node implementations small, testable and independent; each node may be its own module.

---

## 6. resonance-audio-api
Purpose: The stable, public API crate exposing a C-compatible surface; has no internal dependencies (it only defines interfaces and C ABI types).

Key headers (C++):
- `resonance-audio/resonance_audio/api/resonance_audio_api.h`

Dependencies: None (interface-only)

Notes:
- This crate should define the C API types and export the factory function signature.
- If using Rust as the host, this crate can be a tiny C header + bindgen output or a cxx::bridge that defines the ABI.

---

## 7. resonance-audio-engine
Purpose: Concrete implementation that ties everything together and implements the `resonance-audio-api` interfaces.

Key headers (C++):
- `resonance-audio/resonance_audio/graph/resonance_audio_api_impl.h`

Dependencies: resonance-audio-api, resonance-audio-graph, resonance-audio-base, resonance-audio-dsp

Notes:
- This crate holds the `ResonanceAudioApiImpl` implementation and the factory function `CreateResonanceAudioApi`.
- Keep this crate thin: it should act as glue that builds the GraphManager and system settings and forwards calls between the public API and internal crates.

---

## 8. resonance-audio-platform
Purpose: Platform-specific adapters and friendly types for hosts (Unity, FMOD, Wwise, etc.).

Key headers (C++):
- `resonance-audio/resonance_audio/platforms/common/room_properties.h`
- `resonance-audio/resonance_audio/platforms/unity/unity.h`

Dependencies: resonance-audio-api

Notes:
- This crate contains convenience adapters that translate host-friendly types to the engine API. Keep it small and dependency-free beyond the API crate.

---

## Workspace & Cargo layout suggestions

Top-level `Cargo.toml` (workspace) example:

```toml
[workspace]
members = [
  "crates/resonance-audio-base",
  "crates/resonance-audio-dsp",
  "crates/resonance-audio-ambisonics",
  "crates/resonance-audio-acoustics",
  "crates/resonance-audio-graph",
  "crates/resonance-audio-api",
  "crates/resonance-audio-engine",
  "crates/resonance-audio-platform",
]
```

Each crate would have a minimal `Cargo.toml`. For example, `crates/resonance-audio-base/Cargo.toml`:

```toml
[package]
name = "resonance-audio-base"
version = "0.1.0"
edition = "2021"

[dependencies]
# keep minimal
```

For crates that depend on C++ (Embree, existing C++ headers) consider one of the following approaches:
- Keep the C++ implementation and expose a slim C ABI (or use `cxx`/`ffi`) for Rust to call into. Keep the heavy, performance-critical code in C++ and write Rust wrappers for high-level composition.
- Port the code to Rust gradually starting with `resonance-audio-base` and `resonance-audio-dsp` where safe Rust crates and algorithms map well.

---

## Testing and CI suggestions
- Create unit tests for `base` and `dsp` crates first (AudioBuffer invariants, FFT correctness, filter impulse responses).
- Add cross-crate integration tests that build small scenes and process a few buffers, validating deterministic (or toleranced) output.
- For C++ FFI paths, add CI jobs that build the C++ library and run integration tests against the Rust bindings.

---

## Next actions I added to the todo list
- Add crate skeletons (not-started)
- Create a workspace manifest (not-started)

---

If you want, I can now:
- Add these crate skeleton folders and minimal Cargo.toml files to the repo.
- Create a `crates/resonance-audio-engine/src/lib.rs` that FFI-links to the existing C++ `CreateResonanceAudioApi` factory using `cxx` or raw `bindgen` bindings.
- Or, instead, implement the `include/Engine.h` + `src/Engine.cpp` wrapper files in C++ inside this repo as earlier discussed.

Which would you like next?
