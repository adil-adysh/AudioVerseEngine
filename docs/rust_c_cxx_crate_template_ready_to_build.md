# Resonance Audio — Rust + C++ (`cxx`) crate (complete)

A production-ready crate that binds to Google’s **Resonance Audio** C++ API *without* a C shim by using `cxx` and a thin C++ wrapper that owns the real `vraudio::ResonanceAudioApi` via `std::unique_ptr`. We expose safe Rust methods with validation (length checks), RAII, and no UB.

---

## Layout

```
resonance-cxx/
├── Cargo.toml
├── build.rs
├── README.md
├── src/
│   ├── lib.rs
│   └── bridge.rs
└── cxx/
    ├── include/
    │   └── resonance_bridge.h
    └── src/
        └── resonance_bridge.cc
```

> You must have the Resonance Audio headers & libs available. See **Linking** below.

---

## Cargo.toml

```toml
[package]
name = "resonance-cxx"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
description = "Safe Rust bindings to vraudio::ResonanceAudioApi using cxx"

[dependencies]
cxx = "1.0"

[build-dependencies]
cxx-build = "1.0"

[features]
# Default: only interleaved buffer APIs. Planar buffers can be added later.
default = []
```

---

## build.rs

```rust
fn main() {
    // Where is resonance_audio_api.h ?
    // Set env VRAUDIO_INCLUDE to the folder that contains
    //   resonance_audio/api/resonance_audio_api.h
    let vraudio_include = std::env::var("VRAUDIO_INCLUDE")
        .expect("Set VRAUDIO_INCLUDE to the path containing resonance_audio_api.h");

    // (Optional) where to find the built library to link.
    if let Ok(lib_dir) = std::env::var("VRAUDIO_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }
    // (Optional) which library name to link. Default tries `vraudio`.
    let lib_name = std::env::var("VRAUDIO_LIB_NAME").unwrap_or_else(|_| "vraudio".to_string());
    println!("cargo:rustc-link-lib={}
", lib_name);

    cxx_build::bridge("src/bridge.rs")
        .file("cxx/src/resonance_bridge.cc")
        .include("cxx/include")
        .include(vraudio_include)
        .flag_if_supported("-std=c++17")
        .compile("resonance_cxx_bridge");

    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=cxx/include/resonance_bridge.h");
    println!("cargo:rerun-if-changed=cxx/src/resonance_bridge.cc");
}
```

---

## src/bridge.rs (the `cxx::bridge`)

```rust
#[cxx::bridge(namespace = "ra")]
mod ffi {
    // ---------- Rust-owned types shared with C++ ----------
    #[repr(i32)]
    enum RenderingMode {
        kStereoPanning = 0,
        kBinauralLowQuality = 1,
        kBinauralMediumQuality = 2,
        kBinauralHighQuality = 3,
        kRoomEffectsOnly = 4,
    }

    #[repr(i32)]
    enum DistanceRolloffModel {
        kLogarithmic = 0,
        kLinear = 1,
        kNone = 2,
    }

    struct ReflectionProperties {
        room_position: [f32; 3],
        room_rotation: [f32; 4],
        room_dimensions: [f32; 3],
        cutoff_frequency: f32,
        coefficients: [f32; 6],
        gain: f32,
    }

    struct ReverbProperties {
        rt60_values: [f32; 9],
        gain: f32,
    }

    // ---------- Opaque wrapper type that owns the real vraudio API ----------
    unsafe extern "C++" {
        include!("resonance_bridge.h");

        type Api; // ra::Api – a thin C++ wrapper owning std::unique_ptr<vraudio::ResonanceAudioApi>

        // Factory returning std::unique_ptr<Api>
        fn make_api(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> UniquePtr<Api>;

        // Rendering/output into user-provided buffers (validated for size)
        fn fill_interleaved_f32(self: Pin<&mut Api>, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool;
        fn fill_interleaved_i16(self: Pin<&mut Api>, num_channels: usize, num_frames: usize, buffer: &mut [i16]) -> bool;

        // Listener params
        fn set_head_position(self: Pin<&mut Api>, x: f32, y: f32, z: f32);
        fn set_head_rotation(self: Pin<&mut Api>, x: f32, y: f32, z: f32, w: f32);
        fn set_master_volume(self: Pin<&mut Api>, volume: f32);
        fn set_stereo_speaker_mode(self: Pin<&mut Api>, enabled: bool);

        // Sources
        fn create_ambisonic_source(self: Pin<&mut Api>, num_channels: usize) -> i32;
        fn create_stereo_source(self: Pin<&mut Api>, num_channels: usize) -> i32;
        fn create_sound_object_source(self: Pin<&mut Api>, mode: RenderingMode) -> i32;
        fn destroy_source(self: Pin<&mut Api>, source_id: i32);

        // Source buffers (interleaved)
        fn set_interleaved_buffer_f32(self: Pin<&mut Api>, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize);
        fn set_interleaved_buffer_i16(self: Pin<&mut Api>, source_id: i32, audio: &[i16], num_channels: usize, num_frames: usize);

        // Source params
        fn set_source_distance_attenuation(self: Pin<&mut Api>, source_id: i32, distance_attenuation: f32);
        fn set_source_distance_model(self: Pin<&mut Api>, source_id: i32, rolloff: DistanceRolloffModel, min_distance: f32, max_distance: f32);
        fn set_source_position(self: Pin<&mut Api>, source_id: i32, x: f32, y: f32, z: f32);
        fn set_source_room_effects_gain(self: Pin<&mut Api>, source_id: i32, room_effects_gain: f32);
        fn set_source_rotation(self: Pin<&mut Api>, source_id: i32, x: f32, y: f32, z: f32, w: f32);
        fn set_source_volume(self: Pin<&mut Api>, source_id: i32, volume: f32);
        fn set_sound_object_directivity(self: Pin<&mut Api>, source_id: i32, alpha: f32, order: f32);
        fn set_sound_object_listener_directivity(self: Pin<&mut Api>, source_id: i32, alpha: f32, order: f32);
        fn set_sound_object_near_field_effect_gain(self: Pin<&mut Api>, source_id: i32, gain: f32);
        fn set_sound_object_occlusion_intensity(self: Pin<&mut Api>, source_id: i32, intensity: f32);
        fn set_sound_object_spread(self: Pin<&mut Api>, source_id: i32, spread_deg: f32);

        // Environment
        fn enable_room_effects(self: Pin<&mut Api>, enable: bool);
        fn set_reflection_properties(self: Pin<&mut Api>, props: &ReflectionProperties);
        fn set_reverb_properties(self: Pin<&mut Api>, props: &ReverbProperties);
    }
}

pub use ffi::{Api as RawApi, DistanceRolloffModel, ReflectionProperties, RenderingMode, ReverbProperties};

/// Safe, ergonomic Rust wrapper with RAII and validation.
pub struct Api {
    inner: cxx::UniquePtr<RawApi>,
}

impl Api {
    pub fn new(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> Self {
        let inner = ffi::make_api(num_channels, frames_per_buffer, sample_rate_hz);
        assert!(!inner.is_null(), "vraudio::CreateResonanceAudioApi returned null");
        Self { inner }
    }

    #[inline]
    pub fn as_pin_mut(&mut self) -> ::std::pin::Pin<&mut RawApi> {
        self.inner.pin_mut()
    }

    pub fn fill_interleaved_f32(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool {
        assert_eq!(buffer.len(), num_channels * num_frames, "buffer len mismatch: expected {}", num_channels * num_frames);
        ffi::fill_interleaved_f32(self.as_pin_mut(), num_channels, num_frames, buffer)
    }

    pub fn fill_interleaved_i16(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [i16]) -> bool {
        assert_eq!(buffer.len(), num_channels * num_frames, "buffer len mismatch: expected {}", num_channels * num_frames);
        ffi::fill_interleaved_i16(self.as_pin_mut(), num_channels, num_frames, buffer)
    }

    // Listener
    pub fn set_head_position(&mut self, x: f32, y: f32, z: f32) { ffi::set_head_position(self.as_pin_mut(), x, y, z); }
    pub fn set_head_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) { ffi::set_head_rotation(self.as_pin_mut(), x, y, z, w); }
    pub fn set_master_volume(&mut self, volume: f32) { ffi::set_master_volume(self.as_pin_mut(), volume); }
    pub fn set_stereo_speaker_mode(&mut self, enabled: bool) { ffi::set_stereo_speaker_mode(self.as_pin_mut(), enabled); }

    // Sources
    pub fn create_ambisonic_source(&mut self, num_channels: usize) -> i32 { ffi::create_ambisonic_source(self.as_pin_mut(), num_channels) }
    pub fn create_stereo_source(&mut self, num_channels: usize) -> i32 { ffi::create_stereo_source(self.as_pin_mut(), num_channels) }
    pub fn create_sound_object_source(&mut self, mode: RenderingMode) -> i32 { ffi::create_sound_object_source(self.as_pin_mut(), mode) }
    pub fn destroy_source(&mut self, source_id: i32) { ffi::destroy_source(self.as_pin_mut(), source_id); }

    // Buffers
    pub fn set_interleaved_buffer_f32(&mut self, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize) {
        assert_eq!(audio.len(), num_channels * num_frames, "interleaved audio len mismatch");
        ffi::set_interleaved_buffer_f32(self.as_pin_mut(), source_id, audio, num_channels, num_frames)
    }
    pub fn set_interleaved_buffer_i16(&mut self, source_id: i32, audio: &[i16], num_channels: usize, num_frames: usize) {
        assert_eq!(audio.len(), num_channels * num_frames, "interleaved audio len mismatch");
        ffi::set_interleaved_buffer_i16(self.as_pin_mut(), source_id, audio, num_channels, num_frames)
    }

    // Source params
    pub fn set_source_distance_attenuation(&mut self, source_id: i32, distance_attenuation: f32) { ffi::set_source_distance_attenuation(self.as_pin_mut(), source_id, distance_attenuation); }
    pub fn set_source_distance_model(&mut self, source_id: i32, rolloff: DistanceRolloffModel, min_distance: f32, max_distance: f32) { ffi::set_source_distance_model(self.as_pin_mut(), source_id, rolloff, min_distance, max_distance); }
    pub fn set_source_position(&mut self, source_id: i32, x: f32, y: f32, z: f32) { ffi::set_source_position(self.as_pin_mut(), source_id, x, y, z); }
    pub fn set_source_room_effects_gain(&mut self, source_id: i32, room_effects_gain: f32) { ffi::set_source_room_effects_gain(self.as_pin_mut(), source_id, room_effects_gain); }
    pub fn set_source_rotation(&mut self, source_id: i32, x: f32, y: f32, z: f32, w: f32) { ffi::set_source_rotation(self.as_pin_mut(), source_id, x, y, z, w); }
    pub fn set_source_volume(&mut self, source_id: i32, volume: f32) { ffi::set_source_volume(self.as_pin_mut(), source_id, volume); }
    pub fn set_sound_object_directivity(&mut self, source_id: i32, alpha: f32, order: f32) { ffi::set_sound_object_directivity(self.as_pin_mut(), source_id, alpha, order); }
    pub fn set_sound_object_listener_directivity(&mut self, source_id: i32, alpha: f32, order: f32) { ffi::set_sound_object_listener_directivity(self.as_pin_mut(), source_id, alpha, order); }
    pub fn set_sound_object_near_field_effect_gain(&mut self, source_id: i32, gain: f32) { ffi::set_sound_object_near_field_effect_gain(self.as_pin_mut(), source_id, gain); }
    pub fn set_sound_object_occlusion_intensity(&mut self, source_id: i32, intensity: f32) { ffi::set_sound_object_occlusion_intensity(self.as_pin_mut(), source_id, intensity); }
    pub fn set_sound_object_spread(&mut self, source_id: i32, spread_deg: f32) { ffi::set_sound_object_spread(self.as_pin_mut(), source_id, spread_deg); }

    // Environment
    pub fn enable_room_effects(&mut self, enable: bool) { ffi::enable_room_effects(self.as_pin_mut(), enable); }
    pub fn set_reflection_properties(&mut self, props: &ReflectionProperties) { ffi::set_reflection_properties(self.as_pin_mut(), props); }
    pub fn set_reverb_properties(&mut self, props: &ReverbProperties) { ffi::set_reverb_properties(self.as_pin_mut(), props); }
}
```

---

## src/lib.rs

```rust
pub mod bridge;

pub use bridge::{Api, DistanceRolloffModel, ReflectionProperties, RenderingMode, ReverbProperties};
```

---

## cxx/include/resonance_bridge.h

```cpp
#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include "rust/cxx.h"

// Include the original Resonance Audio C++ API
#include "resonance_audio/api/resonance_audio_api.h"

namespace ra {

// A thin owning wrapper around the real vraudio API instance.
class Api {
public:
  explicit Api(std::unique_ptr<vraudio::ResonanceAudioApi> impl) : impl_(std::move(impl)) {}
  Api(Api&&) = default;
  Api& operator=(Api&&) = default;
  Api(const Api&) = delete;
  Api& operator=(const Api&) = delete;
  ~Api() = default;

  vraudio::ResonanceAudioApi* get() { return impl_.get(); }

private:
  std::unique_ptr<vraudio::ResonanceAudioApi> impl_;
};

// Factory
std::unique_ptr<Api> make_api(std::size_t num_channels, std::size_t frames_per_buffer, int sample_rate_hz);

// Rust-side enums/structs are declared in the generated header, but we only
// need free-function wrappers here; definitions live in .cc.

// Output
bool fill_interleaved_f32(Api& api, std::size_t num_channels, std::size_t num_frames, rust::Slice<float> buffer);
bool fill_interleaved_i16(Api& api, std::size_t num_channels, std::size_t num_frames, rust::Slice<int16_t> buffer);

// Listener
void set_head_position(Api& api, float x, float y, float z);
void set_head_rotation(Api& api, float x, float y, float z, float w);
void set_master_volume(Api& api, float volume);
void set_stereo_speaker_mode(Api& api, bool enabled);

// Sources
int create_ambisonic_source(Api& api, std::size_t num_channels);
int create_stereo_source(Api& api, std::size_t num_channels);
int create_sound_object_source(Api& api, int rendering_mode);
void destroy_source(Api& api, int source_id);

// Buffers (interleaved)
void set_interleaved_buffer_f32(Api& api, int source_id, rust::Slice<const float> audio, std::size_t num_channels, std::size_t num_frames);
void set_interleaved_buffer_i16(Api& api, int source_id, rust::Slice<const int16_t> audio, std::size_t num_channels, std::size_t num_frames);

// Source params
void set_source_distance_attenuation(Api& api, int source_id, float distance_attenuation);
void set_source_distance_model(Api& api, int source_id, int rolloff, float min_distance, float max_distance);
void set_source_position(Api& api, int source_id, float x, float y, float z);
void set_source_room_effects_gain(Api& api, int source_id, float room_effects_gain);
void set_source_rotation(Api& api, int source_id, float x, float y, float z, float w);
void set_source_volume(Api& api, int source_id, float volume);
void set_sound_object_directivity(Api& api, int source_id, float alpha, float order);
void set_sound_object_listener_directivity(Api& api, int source_id, float alpha, float order);
void set_sound_object_near_field_effect_gain(Api& api, int source_id, float gain);
void set_sound_object_occlusion_intensity(Api& api, int source_id, float intensity);
void set_sound_object_spread(Api& api, int source_id, float spread_deg);

// Environment
struct ReflectionProperties; // from bridge.rs (generated header)
struct ReverbProperties;     // from bridge.rs (generated header)

void enable_room_effects(Api& api, bool enable);
void set_reflection_properties(Api& api, const ReflectionProperties& props);
void set_reverb_properties(Api& api, const ReverbProperties& props);

} // namespace ra
```

---

## cxx/src/resonance_bridge.cc

```cpp
#include "resonance_bridge.h"
#include "rust/cxx.h"
#include "resonance_audio/api/resonance_audio_api.h"
#include "resonance_bridge.rs.h" // Generated by cxx for enums/structs

#include <cstdio>
#include <utility>

namespace ra {

using vraudio::DistanceRolloffModel;
using vraudio::RenderingMode;

static inline bool check_size(std::size_t channels, std::size_t frames, std::size_t len) {
  const std::size_t need = channels * frames;
  if (len != need) {
    std::fprintf(stderr, "[resonance-cxx] buffer size mismatch: have=%zu need=%zu (ch=%zu, frames=%zu)
", len, need, channels, frames);
    return false;
  }
  return true;
}

std::unique_ptr<Api> make_api(std::size_t num_channels, std::size_t frames_per_buffer, int sample_rate_hz) {
  std::unique_ptr<vraudio::ResonanceAudioApi> impl{
      vraudio::CreateResonanceAudioApi(num_channels, frames_per_buffer, sample_rate_hz)};
  if (!impl) {
    std::fprintf(stderr, "[resonance-cxx] CreateResonanceAudioApi returned null
");
    return nullptr;
  }
  return std::make_unique<Api>(std::move(impl));
}

bool fill_interleaved_f32(Api& api, std::size_t num_channels, std::size_t num_frames, rust::Slice<float> buffer) {
  if (!check_size(num_channels, num_frames, buffer.size())) return false;
  return api.get()->FillInterleavedOutputBuffer(num_channels, num_frames, buffer.data());
}

bool fill_interleaved_i16(Api& api, std::size_t num_channels, std::size_t num_frames, rust::Slice<int16_t> buffer) {
  if (!check_size(num_channels, num_frames, buffer.size())) return false;
  return api.get()->FillInterleavedOutputBuffer(num_channels, num_frames, buffer.data());
}

// Listener
void set_head_position(Api& api, float x, float y, float z) { api.get()->SetHeadPosition(x, y, z); }
void set_head_rotation(Api& api, float x, float y, float z, float w) { api.get()->SetHeadRotation(x, y, z, w); }
void set_master_volume(Api& api, float volume) { api.get()->SetMasterVolume(volume); }
void set_stereo_speaker_mode(Api& api, bool enabled) { api.get()->SetStereoSpeakerMode(enabled); }

// Sources
int create_ambisonic_source(Api& api, std::size_t num_channels) { return api.get()->CreateAmbisonicSource(num_channels); }
int create_stereo_source(Api& api, std::size_t num_channels) { return api.get()->CreateStereoSource(num_channels); }
int create_sound_object_source(Api& api, int mode) { return api.get()->CreateSoundObjectSource(static_cast<RenderingMode>(mode)); }
void destroy_source(Api& api, int source_id) { api.get()->DestroySource(source_id); }

// Buffers (interleaved)
void set_interleaved_buffer_f32(Api& api, int source_id, rust::Slice<const float> audio, std::size_t num_channels, std::size_t num_frames) {
  if (!check_size(num_channels, num_frames, audio.size())) return;
  api.get()->SetInterleavedBuffer(source_id, audio.data(), num_channels, num_frames);
}
void set_interleaved_buffer_i16(Api& api, int source_id, rust::Slice<const int16_t> audio, std::size_t num_channels, std::size_t num_frames) {
  if (!check_size(num_channels, num_frames, audio.size())) return;
  api.get()->SetInterleavedBuffer(source_id, audio.data(), num_channels, num_frames);
}

// Source params
void set_source_distance_attenuation(Api& api, int source_id, float distance_attenuation) { api.get()->SetSourceDistanceAttenuation(source_id, distance_attenuation); }
void set_source_distance_model(Api& api, int source_id, int rolloff, float min_distance, float max_distance) { api.get()->SetSourceDistanceModel(source_id, static_cast<DistanceRolloffModel>(rolloff), min_distance, max_distance); }
void set_source_position(Api& api, int source_id, float x, float y, float z) { api.get()->SetSourcePosition(source_id, x, y, z); }
void set_source_room_effects_gain(Api& api, int source_id, float room_effects_gain) { api.get()->SetSourceRoomEffectsGain(source_id, room_effects_gain); }
void set_source_rotation(Api& api, int source_id, float x, float y, float z, float w) { api.get()->SetSourceRotation(source_id, x, y, z, w); }
void set_source_volume(Api& api, int source_id, float volume) { api.get()->SetSourceVolume(source_id, volume); }
void set_sound_object_directivity(Api& api, int source_id, float alpha, float order) { api.get()->SetSoundObjectDirectivity(source_id, alpha, order); }
void set_sound_object_listener_directivity(Api& api, int source_id, float alpha, float order) { api.get()->SetSoundObjectListenerDirectivity(source_id, alpha, order); }
void set_sound_object_near_field_effect_gain(Api& api, int source_id, float gain) { api.get()->SetSoundObjectNearFieldEffectGain(source_id, gain); }
void set_sound_object_occlusion_intensity(Api& api, int source_id, float intensity) { api.get()->SetSoundObjectOcclusionIntensity(source_id, intensity); }
void set_sound_object_spread(Api& api, int source_id, float spread_deg) { api.get()->SetSoundObjectSpread(source_id, spread_deg); }

// Environment
void enable_room_effects(Api& api, bool enable) { api.get()->EnableRoomEffects(enable); }

void set_reflection_properties(Api& api, const ReflectionProperties& p) {
  vraudio::ReflectionProperties rp;
  rp.room_position[0] = p.room_position[0];
  rp.room_position[1] = p.room_position[1];
  rp.room_position[2] = p.room_position[2];
  rp.room_rotation[0] = p.room_rotation[0];
  rp.room_rotation[1] = p.room_rotation[1];
  rp.room_rotation[2] = p.room_rotation[2];
  rp.room_rotation[3] = p.room_rotation[3];
  rp.room_dimensions[0] = p.room_dimensions[0];
  rp.room_dimensions[1] = p.room_dimensions[1];
  rp.room_dimensions[2] = p.room_dimensions[2];
  rp.cutoff_frequency = p.cutoff_frequency;
  for (int i = 0; i < 6; ++i) rp.coefficients[i] = p.coefficients[ i ];
  rp.gain = p.gain;
  api.get()->SetReflectionProperties(rp);
}

void set_reverb_properties(Api& api, const ReverbProperties& p) {
  vraudio::ReverbProperties rp;
  for (int i = 0; i < 9; ++i) rp.rt60_values[i] = p.rt60_values[i];
  rp.gain = p.gain;
  api.get()->SetReverbProperties(rp);
}

} // namespace ra
```

---

## README.md

```markdown
# Resonance Audio — Rust + C++ (`cxx`) crate

This crate gives you safe Rust bindings to `vraudio::ResonanceAudioApi` without a C shim.

## Requirements

- C++17 toolchain
- The Resonance Audio headers and library built somewhere on disk.
  - Set `VRAUDIO_INCLUDE` to the directory that contains `resonance_audio/api/resonance_audio_api.h`.
  - Optionally set `VRAUDIO_LIB_DIR` (for linker search path) and `VRAUDIO_LIB_NAME` (defaults to `vraudio`).

## Example

```rust
use resonance_cxx::{Api, RenderingMode};

fn main() {
    let mut api = Api::new(2, 512, 48000);
    api.set_master_volume(1.0);

    let src = api.create_sound_object_source(RenderingMode::kBinauralHighQuality);
    let mut out = vec![0.0f32; 2 * 512];
    let ok = api.fill_interleaved_f32(2, 512, &mut out);
    println!("rendered: {}", ok);

    api.destroy_source(src);
}
```

## Notes on safety

- All rendering/buffer functions validate slice length = `num_channels * num_frames`.
- The underlying API instance is owned by RAII (`std::unique_ptr`) on the C++ side and by `UniquePtr` on the Rust side.
- Mutating calls require a pinned `&mut` to avoid aliasing UB.
- We mirror enums and POD structs; conversions are explicit.

## Planar buffers

Planar buffer helpers can be added by constructing a temporary pointer array on the C++ side and exposing a wrapper taking a `&[&[T]]`-like representation from Rust. If you want that, we can extend the bridge in a follow-up.
```

---

### Linking

You’ll typically build the Resonance Audio library separately (e.g., as `libvraudio.a`/`.so`). Then:

```bash
export VRAUDIO_INCLUDE=/path/to/resonance-audio
export VRAUDIO_LIB_DIR=/path/to/build/lib
export VRAUDIO_LIB_NAME=vraudio   # or your actual lib name
cargo build
```

