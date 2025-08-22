## Unity plugin to Rust mapping (Renderer & Spatializer)

Purpose
- Map the Unity native plugin C++ surface (platforms/unity) into equivalent Rust `resonance-cxx` patterns so maintainers can see the direct translation used by the `cxx` bridge and where to add features safely.

Scope
- Covers files under `resonance-audio/platforms/unity/`.
- Produces a one-to-one mapping for types, enums, structs, functions and plugin callbacks into Rust-friendly patterns (opaque types, enums with `#[repr(i32)]`, POD structs, `UniquePtr` factories, `&[T]`/`rust::Slice` vs pointer arrays for planar buffers).

Notes and conventions
- C++ classes exposed via the `cxx` bridge are represented as opaque `pub type` declarations in Rust inside a `#[cxx::bridge(namespace = "ra")]` module.
- C++ enums that have explicit integer representations are declared in Rust with `#[repr(i32)]` and matching discriminant names.
- Interleaved buffers map to Rust slices (`&[f32]` / `&mut [f32]` or `rust::Slice<T>` on the C++ side). Planar buffers map to pointer-to-pointer signatures in C++ and to `*const *const T` or `*const *mut T` in Rust `unsafe` functions — be exact about constness.

Files audited
- `resonance-audio/platforms/unity/unity.h`
- `resonance-audio/platforms/unity/unity.cc`
- `resonance-audio/platforms/unity/unity_nativeaudioplugins.h`
- `resonance-audio/platforms/unity/unity_nativeaudioplugins.cc`
- `resonance-audio/platforms/unity/unity_reverb_computer.h`
- `resonance-audio/platforms/unity/unity_reverb_computer.cc`

Top-level C++ surface (Unity plugin)
- Namespace: `vraudio::unity`
- Singleton: `ResonanceAudioSystem` (internal), holds `std::unique_ptr<ResonanceAudioApi> api` and recording state.

- Public functions (C++ signatures) and Rust mapping:
  - void Initialize(int sample_rate, size_t num_channels, size_t frames_per_buffer)
    - Rust: fn create_resonance_audio_api(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> UniquePtr<ResonanceAudioApi>
    - Contract: creates the internal API factory with the same args; map to factory returning `UniquePtr` in `bridge.rs`.

  - void Shutdown()
    - Rust: Drop the `UniquePtr<ResonanceAudioApi>` by letting it fall out of scope or calling `unique_ptr::drop` via cxx.

  - void ProcessListener(size_t num_frames, float* output)
    - Rust: `fn fill_interleaved_output_buffer_f32(self: Pin<&mut ResonanceAudioApi>, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool`
    - Notes: Called on audio thread; Rust binding should be `&mut [f32]` for interleaved output.

  - void SetListenerTransform(float px, float py, float pz, float qx, float qy, float qz, float qw)
    - Rust: `fn set_head_position(self: Pin<&mut ResonanceAudioApi>, x:f32,y:f32,z:f32)` and `fn set_head_rotation(self: Pin<&mut ResonanceAudioApi>, x:f32,y:f32,z:f32,w:f32)`.

  - Source lifecycle & processing
    - ResonanceAudioApi::SourceId CreateSoundfield(int num_channels)
      - Rust: `fn create_ambisonic_source(self: Pin<&mut ResonanceAudioApi>, num_channels: usize) -> i32`
    - ResonanceAudioApi::SourceId CreateSoundObject(RenderingMode rendering_mode)
      - Rust: `fn create_sound_object_source(self: Pin<&mut ResonanceAudioApi>, rendering_mode: RenderingMode) -> i32`
    - void DestroySource(SourceId id)
      - Rust: `fn destroy_source(self: Pin<&mut ResonanceAudioApi>, id: i32)`
    - void ProcessSource(SourceId id, size_t num_channels, size_t num_frames, float* input)
      - Rust: `fn set_interleaved_buffer_f32(self: Pin<&mut ResonanceAudioApi>, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize)`
      - Notes: Unity calls `SetInterleavedBuffer` from audio thread with `float* input`.

  - Planar buffers
    - FillPlanarOutputBuffer(size_t num_channels,size_t num_frames,float* const* buffer_ptr)
      - Rust: `unsafe fn fill_planar_output_buffer_f32(self: Pin<&mut ResonanceAudioApi>, num_channels: usize, num_frames: usize, buffers: *const *mut f32) -> bool`
      - IMPORTANT: the `float* const*` (C++) means an array of pointers to mutable floats (channels) where the array pointer is const; in Rust this maps to `*const *mut f32` and must be `unsafe`. Match constness exactly.
    - SetPlanarBuffer(SourceId, const float* const* audio_buffer_ptr, ...)
      - Rust: `unsafe fn set_planar_buffer_f32_ptrs(self: Pin<&mut ResonanceAudioApi>, source_id: i32, audio_ptrs: *const *const f32, num_channels: usize, num_frames: usize)`
      - Notes: pointer-to-pointer to const float when audio is immutable.

  - Source setters (directivity, distance, gain, occlusion, room effects, spread, transform)
    - Map to `fn SetSourceX(self: Pin<&mut ResonanceAudioApi>, source_id: i32, ... )` functions in Rust using primitive f32s and enums where applicable (DistanceRolloffModel, RenderingMode).

Unity native audio plugin callbacks (Renderer and Spatializer)
- Renderer (non-spatialized renderer plugin) — acts as a simple renderer that calls into `vraudio::unity::Initialize`, `ProcessListener`, `Shutdown`.
  - RendererCreateCallback(UnityAudioEffectState* state)
    - Rust equivalent: plugin initialization flows to `create_resonance_audio_api(...)` at startup.
  - RendererProcessCallback(UnityAudioEffectState*, float* inbuffer, float* outbuffer, unsigned int length, int inchannels, int outchannels)
    - Rust: call into `FillInterleavedOutputBuffer` (interleaved) and copy/zero if no valid output.

- Spatializer (per-source spatializer plugin) — acts as a source-level spatializer and ambisonic decoder; responsible for creating/destroying sources and updating per-source parameters.
  - SpatializerCreateCallback / SpatializerReleaseCallback
    - Map to `CreateSoundObject`/`DestroySource` in Rust via bridge.
  - SpatializerProcessCallback(UnityAudioEffectState*, float* inbuffer, float* outbuffer, unsigned int length, int inchannels, int outchannels)
    - Map to: build listener/source transforms (Eigen math is done in C++), then call `ProcessSource` which ultimately calls `SetInterleavedBuffer` and then Unity expects the plugin to write output (Unity copies input out later). In Rust the per-source setter is `SetInterleavedBufferF32`.
  - SpatializerSetFloatParameterCallback/SpatializerGetFloatParameterCallback
    - Map to Rust setter methods for source parameters (volume/gain/quality/occlusion/directivity/etc.). When quality changes, Unity destroys & recreates the source — the bridge should accept `CreateSoundObjectSource(rendering_mode)`.

Enums and PODs mapping
- RenderingMode (C++ enum vraudio::RenderingMode) -> Rust `#[repr(i32)] pub enum RenderingMode { ... }`
- DistanceRolloffModel -> Rust `#[repr(i32)] pub enum DistanceRolloffModel { ... }`
- ReflectionProperties (POD) -> Rust `pub struct ReflectionProperties { room_position: [f32;3], room_rotation:[f32;4], room_dimensions:[f32;3], cutoff_frequency:f32, coefficients:[f32;6], gain:f32 }` (matching layout)
- ReverbProperties -> Rust `pub struct ReverbProperties { rt60_values:[f32;9], gain:f32 }`

Bridging patterns (practical tips)
- Use `UniquePtr<ResonanceAudioApi>` factory emitted by `CreateResonanceAudioApi` in `bridge.rs`.
- Match pointer-to-pointer constness exactly between `api.h` wrapper and Rust `bridge.rs` declarations; cxx-generated glue is sensitive to member function pointer types.
- For interleaved read-only audio passed to C++ implementers, use `::rust::Slice<const T>` on the C++ wrapper and `&[T]` on the Rust side.
- For audio output where C++ writes into buffers supplied by the caller (Planar output), use `float* const*` on C++ and `*const *mut f32` in Rust for `unsafe` functions.

Where to find bridge examples in this repo
- `resonance-cxx/src/bridge.rs` — existing bridge exposing many public API methods (use this as canonical Rust mapping)
- `resonance-cxx/cxx/include/resonance_bridge.h` and `resonance-cxx/cxx/src/resonance_api_wrapper.cc` — wrapper implementations and pointer conversions.

Gaps & TODOs (future work)
- Ambisonic recorder access: Unity uses an internal-only `ResonanceAudioApiImpl::GetAmbisonicOutputBuffer()` for the desktop recorder; consider exposing a safe API or a one-off testing-only bridge method if you want to access the ambisonic buffer from Rust.
- Add Rust-side tests that exercise planar pointer APIs and spatializer lifecycle.
- Consider adding a small Rust helper wrapper that provides safe planar-slice builders for callers instead of raw pointer arrays.

References
- `resonance-audio/platforms/unity/unity.h`
- `resonance-audio/platforms/unity/unity.cc`
- `resonance-audio/platforms/unity/unity_nativeaudioplugins.h`
- `resonance-audio/platforms/unity/unity_nativeaudioplugins.cc`
- `resonance-cxx/src/bridge.rs`

Completion
- Mapped Unity renderer and spatializer plugin surfaces to Rust `cxx` patterns and types. Use this document as the authoritative quick-reference when adding new `cxx::bridge` methods for Unity-used features.
