
# Crate Requirements: `resonance-audio-engine`

## Goal

Provide a Rust crate exposing a **Renderer** (global audio output) and **Spatializer** (per-source processing) inspired by Unity’s plugin model, but independent of Unity headers.
This acts as the idiomatic Rust layer for real-time audio games or middleware.

---

## Concepts

* **Renderer**

  * Global output processor.
  * Owns a `ResonanceAudioApi` instance (`UniquePtr`).
  * Responsible for: initialization, listener transform, mixing into interleaved/planar buffers.

* **Spatializer**

  * Per-source processor bound to a `SourceId`.
  * Responsible for: creating/destroying sound object sources, feeding interleaved/planar buffers, updating per-source parameters.

* **Separation of Concerns**

  * Renderer ↔ global mix.
  * Spatializer ↔ individual sources.
  * Both implemented as thin wrappers over `resonance-cxx`.

---

## Crate Layout

```
resonance-audio-engine/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── bridge.rs        // cxx::bridge definitions
│   ├── renderer.rs      // Safe Renderer API
│   ├── spatializer.rs   // Safe Spatializer API
│   ├── types.rs         // Enums, PODs, helpers
│   └── utils.rs         // safe buffer helpers
├── cxx/
│   ├── include/resonance_unity_bridge.h   // wrapper headers (non-Unity-specific)
│   └── src/resonance_unity_wrapper.cc     // C++ forwarding impls
```

---

## Rust API (High-Level)

### Renderer

```rust
pub struct Renderer {
    api: cxx::UniquePtr<ffi::ResonanceAudioApi>,
}

impl Renderer {
    pub fn new(sample_rate_hz: i32, num_channels: usize, frames_per_buffer: usize) -> Self;
    pub fn process_output(&mut self, buffer: &mut [f32], num_channels: usize, num_frames: usize) -> bool;
    pub fn process_output_planar(&mut self, buffers: *const *mut f32, num_channels: usize, num_frames: usize) -> bool;
    pub fn set_listener_position(&mut self, x: f32, y: f32, z: f32);
    pub fn set_listener_rotation(&mut self, x: f32, y: f32, z: f32, w: f32);
}
```

### Spatializer

```rust
pub struct Spatializer {
    api: cxx::UniquePtr<ffi::ResonanceAudioApi>, // ref to Renderer-owned api
    source_id: i32,
}

impl Spatializer {
    pub fn new(api: &mut Renderer, rendering_mode: RenderingMode) -> Self;
    pub fn feed_interleaved(&mut self, audio: &[f32], num_channels: usize, num_frames: usize);
    pub fn feed_planar(&mut self, audio_ptrs: *const *const f32, num_channels: usize, num_frames: usize);
    pub fn set_gain(&mut self, gain: f32);
    pub fn set_distance_rolloff(&mut self, model: DistanceRolloffModel);
    pub fn destroy(self, api: &mut Renderer);
}
```

---

## Types

* **Enums**

  ```rust
  #[repr(i32)]
  pub enum RenderingMode { Stereo = 0, Binaural = 1, ... }

  #[repr(i32)]
  pub enum DistanceRolloffModel { Logarithmic = 0, Linear = 1, None = 2 }
  ```
* **Structs (POD)**

  ```rust
  #[repr(C)]
  pub struct ReflectionProperties {
      pub room_position: [f32; 3],
      pub room_rotation: [f32; 4],
      pub room_dimensions: [f32; 3],
      pub cutoff_frequency: f32,
      pub coefficients: [f32; 6],
      pub gain: f32,
  }

  #[repr(C)]
  pub struct ReverbProperties {
      pub rt60_values: [f32; 9],
      pub gain: f32,
  }
  ```

---

## Safety Conventions

* **Interleaved Buffers**: `&mut [f32]` (safe).
* **Planar Buffers**: `*const *mut f32` / `*const *const f32` (unsafe).
* **Drop**: `UniquePtr` handles RAII cleanup.

---

## Tests

* Renderer lifecycle (create, process, drop).
* Spatializer lifecycle (create, feed, drop).
* Planar buffer roundtrip (unsafe).
* Parameter setters (gain, distance, etc.).

---

## Future Work

* Ambisonic recorder exposure.
* Thread-safe helpers for planar buffers.
* Optional DSP graph integration.
