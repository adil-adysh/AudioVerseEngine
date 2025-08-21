# Engine Headers Reference

This document lists the most relevant headers from the `resonance-audio` codebase that are useful to implement a minimal, engine-level audio game engine. For each header the key classes/types/method signatures are listed and a short note explains the purpose and recommended usage.

Notes:
- Focus is engine-level only: initialization, listener, sources, rooms, geometry, and audio processing.
- Use `resonance_audio_api.h` as the main public surface unless you want to expose your own wrapper. Many headers below are internal implementation but are good references or reuse candidates.

---

## Core public API

### `resonance-audio/resonance_audio/api/resonance_audio_api.h`
Purpose: Primary public API. Exposes engine creation, listener, sources, room effects, and buffer IO.

Key types & signatures:

- Factory
  - extern "C" EXPORT_API ResonanceAudioApi* CreateResonanceAudioApi(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz);
    - Creates engine instance. Caller owns returned pointer and must delete when done.

- Enums
  - enum RenderingMode { kStereoPanning, kBinauralLowQuality, kBinauralMediumQuality, kBinauralHighQuality, kRoomEffectsOnly };
  - enum DistanceRolloffModel { kLogarithmic, kLinear, kNone };

- Structs
  - struct ReflectionProperties { float room_position[3]; float room_rotation[4]; float room_dimensions[3]; float cutoff_frequency; float coefficients[6]; float gain; };
  - struct ReverbProperties { float rt60_values[9]; float gain; };

- Main API class (abstract): `class ResonanceAudioApi`
  - typedef int SourceId; static const SourceId kInvalidSourceId = -1;
  - virtual bool FillInterleavedOutputBuffer(size_t num_channels, size_t num_frames, float* buffer_ptr) = 0;
  - virtual bool FillInterleavedOutputBuffer(size_t num_channels, size_t num_frames, int16* buffer_ptr) = 0;
  - virtual bool FillPlanarOutputBuffer(size_t num_channels, size_t num_frames, float* const* buffer_ptr) = 0;
  - virtual bool FillPlanarOutputBuffer(size_t num_channels, size_t num_frames, int16* const* buffer_ptr) = 0;
  - virtual void SetHeadPosition(float x, float y, float z) = 0;
  - virtual void SetHeadRotation(float x, float y, float z, float w) = 0;
  - virtual void SetMasterVolume(float volume) = 0;
  - virtual void SetStereoSpeakerMode(bool enabled) = 0;
  - virtual SourceId CreateAmbisonicSource(size_t num_channels) = 0;
  - virtual SourceId CreateStereoSource(size_t num_channels) = 0;
  - virtual SourceId CreateSoundObjectSource(RenderingMode rendering_mode) = 0;
  - virtual void DestroySource(SourceId id) = 0;
  - virtual void SetInterleavedBuffer(SourceId source_id, const float* audio_buffer_ptr, size_t num_channels, size_t num_frames) = 0;
  - virtual void SetPlanarBuffer(SourceId source_id, const float* const* audio_buffer_ptr, size_t num_channels, size_t num_frames) = 0;
  - virtual void SetSourceDistanceAttenuation(SourceId source_id, float distance_attenuation) = 0;
  - virtual void SetSourceDistanceModel(SourceId source_id, DistanceRolloffModel rolloff, float min_distance, float max_distance) = 0;
  - virtual void SetSourcePosition(SourceId source_id, float x, float y, float z) = 0;
  - virtual void SetSourceRoomEffectsGain(SourceId source_id, float room_effects_gain) = 0;
  - virtual void SetSourceRotation(SourceId source_id, float x, float y, float z, float w) = 0;
  - virtual void SetSourceVolume(SourceId source_id, float volume) = 0;
  - virtual void SetSoundObjectDirectivity(SourceId sound_object_source_id, float alpha, float order) = 0;
  - virtual void SetSoundObjectListenerDirectivity(SourceId sound_object_source_id, float alpha, float order) = 0;
  - virtual void SetSoundObjectNearFieldEffectGain(SourceId sound_object_source_id, float gain) = 0;
  - virtual void SetSoundObjectOcclusionIntensity(SourceId sound_object_source_id, float intensity) = 0;
  - virtual void SetSoundObjectSpread(SourceId sound_object_source_id, float spread_deg) = 0;
  - virtual void EnableRoomEffects(bool enable) = 0;
  - virtual void SetReflectionProperties(const ReflectionProperties& reflection_properties) = 0;
  - virtual void SetReverbProperties(const ReverbProperties& reverb_properties) = 0;

Notes: This header is the canonical public surface to implement or wrap. It supplies the minimal operations necessary for engine binding and can be used as the engine's external API.

---

## Engine core & graph (internal but reusable)

### `resonance-audio/resonance_audio/graph/resonance_audio_api_impl.h`
Purpose: Concrete implementation of `ResonanceAudioApi`. Shows patterns for audio-thread-safe setters (task queue) and graph processing.

Key classes/methods:
- class ResonanceAudioApiImpl : public ResonanceAudioApi
  - ResonanceAudioApiImpl(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz);
  - ~ResonanceAudioApiImpl() override;
  - Overrides for all `ResonanceAudioApi` virtuals (see above signatures).
  - const AudioBuffer* GetAmbisonicOutputBuffer() const;
  - const AudioBuffer* GetStereoOutputBuffer() const;
  - void ProcessNextBuffer();  // executes task queue and triggers GraphManager::Process()

Related internals used by this class:
- `graph/graph_manager.h` — manages nodes and mixing graph; provides CreateAmbisonicSource/CreateStereoSource/CreateSoundObjectSource, Process(), UpdateRoomReflections(), UpdateRoomReverb(), GetAmbisonicBuffer(), GetStereoBuffer().
- `graph/system_settings.h` — stores global params and `SourceParametersManager` which holds per-source parameter states.
- `utils/lockless_task_queue.h` — used to post setter tasks from main thread to be executed on audio thread.

Recommendation: Use `ResonanceAudioApiImpl` as a reference for implementing EngineHandle that hides implementation details and ensures thread-safety.

### `resonance-audio/resonance_audio/graph/graph_manager.h`
Purpose: Builds and processes the audio graph. Provides source lifecycle helpers and access to output buffers.

Key methods (summary):
- GraphManager(const SystemSettings& system_settings);
- void Process();
- AudioBuffer* GetMutableAudioBuffer(SourceId source_id);
- void CreateAmbisonicPannerSource(SourceId id, bool enable_hrtf);
- void CreateStereoSource(SourceId id);
- void CreateAmbisonicSource(SourceId id, size_t num_channels);
- void CreateSoundObjectSource(SourceId id, int ambisonic_order, bool enable_hrtf, bool enable_direct_rendering);
- void DestroySource(SourceId id);
- const AudioBuffer* GetAmbisonicBuffer() const;
- const AudioBuffer* GetStereoBuffer() const;
- void UpdateRoomReflections(); void UpdateRoomReverb();

Recommendation: Use this manager internally to implement ProcessAudio and source management.

---

## Geometry & propagation (room acoustics & occlusion)

### `resonance-audio/resonance_audio/geometrical_acoustics/scene_manager.h`
Purpose: Build Embree scenes from triangle meshes, associate reflection kernels to triangle sets, and build a listener-only scene for fast listener-sphere queries.

Key methods:
- SceneManager(); ~SceneManager();
- void BuildScene(const std::vector<Vertex>& vertex_buffer, const std::vector<Triangle>& triangle_buffer);
- bool AssociateReflectionKernelToTriangles(const ReflectionKernel& reflection_kernel, const std::unordered_set<unsigned int>& triangle_indices);
- const ReflectionKernel& GetAssociatedReflectionKernel(unsigned int triangle_index) const;
- void BuildListenerScene(const std::vector<AcousticListener>& listeners, float listener_sphere_radius);
- size_t GetListenerIndexFromSphereId(unsigned int sphere_id);
- RTCScene scene() const; RTCScene listener_scene() const;

Recommendation: Wrap this into a `CreateGeometry` / `SetGeometryTransform` engine API. Use `BuildScene` to upload mesh triangles.

### `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_source.h`
Purpose: Models a point sound source used for geometric acoustics; provides ray sampling utilities.

Key usage:
- AcousticSource(const Eigen::Vector3f& position, const std::array<float,kNumReverbOctaveBands>& energies, RNG);
- AcousticRay GenerateRay() const;
- std::vector<AcousticRay> GenerateStratifiedRays(size_t num_rays, size_t sqrt_num_rays) const;

Recommendation: Expose SourceConfig with an optional propagation flag; use this class in the propagation pipeline.

### `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_listener.h`
Purpose: Listener data for impulse-response collection; stores per-band energy impulse responses.

Key struct: `struct AcousticListener { Eigen::Vector3f position; std::array<std::vector<float>,kNumReverbOctaveBands> energy_impulse_responses; }`

Recommendation: Internal structure for IR computation.

### Propagation engine (path tracer, IR computer, reflection kernel)
- `geometrical_acoustics/path_tracer.h` / `impulse_response_computer.h` / `reflection_kernel.h` / `proxy_room_estimator.h`
  - Purpose: Full ray-tracing and IR/RT60 estimation pipeline. Use to compute reverb/reflection properties from geometry.
  - Recommendation: Expose simplified helpers like `ComputeRoomRT60(const Geometry&, const RoomProperties&)` or `EstimateReflections(...)` rather than exposing detailed ray-tracing to game code.

---

## Room description & platform-friendly structs

### `resonance-audio/resonance_audio/platforms/common/room_properties.h`
Purpose: Platform-friendly RoomProperties struct with per-surface material enums, dimensions, reverb tuning parameters.

Key type:
- enum MaterialName { kTransparent, kAcousticCeilingTiles, ..., kNumMaterialNames };
- struct RoomProperties { float position[3]; float rotation[4]; float dimensions[3]; MaterialName material_names[6]; float reflection_scalar; float reverb_gain; float reverb_time; float reverb_brightness; };

Recommendation: Use this struct as the engine-level RoomConfig for CreateRoom/SetRoomProperties; it maps easily to `ReflectionProperties` and `ReverbProperties` used by the DSP.

---

## Audio buffers & utils

### `resonance-audio/resonance_audio/base/audio_buffer.h`
Purpose: Planar multi-channel audio buffer container with aligned allocations and Channel views.

Key methods & types:
- AudioBuffer(size_t num_channels, size_t num_frames);
- ChannelView operator[](size_t channel);
- size_t num_channels() const; size_t num_frames() const; void Clear();
- set_source_id(SourceId id);

Recommendation: Use internally as canonical buffer format. Provide engine-level input adapters that convert interleaved or streaming data into this format.

### `resonance-audio/resonance_audio/utils/lockless_task_queue.h`
Purpose: Thread-safe producer / single-consumer task queue used for posting parameter updates from main thread to audio thread.

Key methods:
- explicit LocklessTaskQueue(size_t max_tasks);
- void Post(Task&& task);
- void Execute(); void Clear();

Recommendation: Use for real-time-safe setter posting in the engine wrapper.

---

## Ambisonics / binaural decoding

### `resonance-audio/resonance_audio/ambisonics/ambisonic_binaural_decoder.h`
Purpose: Decode Ambisonic soundfield to binaural stereo via convolution with SH-HRIRs.

Key signature:
- AmbisonicBinauralDecoder(const AudioBuffer& sh_hrirs, size_t frames_per_buffer, FftManager* fft_manager);
- void Process(const AudioBuffer& input, AudioBuffer* output);

Recommendation: Expose ambisonic soundfield support in engine optionally (CreateSoundfield API). Use this decoder inside GraphManager.

---

## Platform integration example

### `resonance-audio/resonance_audio/platforms/unity/unity.h`
Purpose: Unity-specific adapter wrapping the public API for Unity plugin use.

Key functions:
- void Initialize(int sample_rate, size_t num_channels, size_t frames_per_buffer);
- void Shutdown();
- void ProcessListener(size_t num_frames, float* output);  // called from audio thread
- void SetListenerTransform(float px, float py, float pz, float qx, float qy, float qz, float qw);
- ResonanceAudioApi::SourceId CreateSoundfield(int num_channels);
- ResonanceAudioApi::SourceId CreateSoundObject(RenderingMode rendering_mode);
- void DestroySource(ResonanceAudioApi::SourceId id);
- void ProcessSource(ResonanceAudioApi::SourceId id, size_t num_channels, size_t num_frames, float* input);
- void SetSourceTransform(...), SetSourceGain(...), SetSourceDirectivity(...), etc.
- extern "C" exported helpers: SetListenerGain, SetListenerStereoSpeakerMode, SetRoomProperties(RoomProperties*, float* rt60s), Start/Stop recorder.

Recommendation: Use `unity.h` as a concrete example of a thin adapter that implements the engine public surface expected by a game engine host.

---

## Source parameter manager

### `resonance-audio/resonance_audio/graph/source_parameters_manager.h`
Purpose: Storage and processing of per-source parameters; lets the audio thread process parameter updates in a single pass.

Key methods:
- void Register(SourceId source_id);
- void Unregister(SourceId source_id);
- const SourceParameters* GetParameters(SourceId source_id) const;
- void ProcessAllParameters(const Process& process);

Recommendation: Mirror this pattern in your engine to keep setter complexity off the audio thread.

---

## Quick mapping summary (minimal engine API → resonance-audio components)
- CreateEngine(...) → `CreateResonanceAudioApi(...)` (or a wrapper around `ResonanceAudioApiImpl`).
- DestroyEngine(...) → delete api instance.
- UpdateEngine/ProcessAudio(...) → `FillInterleavedOutputBuffer` / `FillPlanarOutputBuffer`.
- CreateListener(...) + SetListenerTransform(...) → `SetHeadPosition` + `SetHeadRotation`.
- CreateSource(...) + SetSourceAudio(...) → `CreateSoundObjectSource` / `CreateAmbisonicSource` / `CreateStereoSource` + `SetInterleavedBuffer` / `SetPlanarBuffer`.
- DestroySource(...) → `DestroySource`.
- CreateRoom(...) / SetRoomProperties(...) → `SetReflectionProperties` + `SetReverbProperties` (and optionally geometry via `SceneManager::BuildScene`).
- CreateGeometry(...) / SetGeometryTransform(...) → `SceneManager::BuildScene` and reflection-kernel association helpers.

---

## Next steps / Implementation suggestions
- Implement a thin Engine wrapper header (`Engine.h`) that exposes the minimal API you defined (CreateEngine/DestroyEngine/UpdateEngine, Listener, Sources, Rooms, Geometry). Internally hold `std::unique_ptr<ResonanceAudioApi>` and use the provided APIs.
- Adopt `LocklessTaskQueue` pattern to post setters to audio thread, or reuse `ResonanceAudioApiImpl` which already performs this.
- Expose in-engine conversion helpers for interleaved ↔ planar formats (`utils/planar_interleaved_conversion.h`).
- Provide adapter utilities that convert `RoomProperties` → `ReflectionProperties` and `ReverbProperties`.

---

If you want, I can now:
- Create the `Engine.h` wrapper and a minimal `Engine.cpp` that forwards to `ResonanceAudioApi`.
- Produce a mapping example: "Implement a house with multiple rooms using only the minimal API".
- Expand this doc with an exhaustive list of all headers and one-line summaries.

Which one do you want next?

---

## Minimal Engine wrapper (suggested)

Purpose: a thin, engine-level wrapper (`Engine.h` / `Engine.cpp`) that hides `ResonanceAudioApi` internals and provides the minimal public API you requested. This wrapper forwards calls to `ResonanceAudioApi` and handles thread-safe setter posting and buffer format conversions.

Design notes:
- The wrapper holds an instance of `ResonanceAudioApi*` (created via `CreateResonanceAudioApi`).
- Public setters post updates to a lockless task queue or directly call the API where safe. Processing is done on the audio thread by calling `ProcessAudio` / `UpdateEngine` which invokes `FillInterleavedOutputBuffer`.
- The wrapper provides simple Start/Stop/Play semantics (optional) by controlling whether buffers are fed.

Suggested header: `include/Engine.h`

```cpp
// Minimal engine wrapper header (example)
#pragma once
#include <cstddef>
#include <cstdint>
#include "api/resonance_audio_api.h"

namespace audioengine {

using EngineHandle = struct Engine*; // opaque in C ABI if needed

struct Vec3 { float x, y, z; };
struct Quat { float x, y, z, w; };

class Engine {
 public:
  // Create/destroy engine. Returns nullptr on failure.
  static Engine* Create(size_t num_output_channels, size_t frames_per_buffer, int sample_rate_hz);
  static void Destroy(Engine* engine);

  // Run one audio processing step; fills interleaved float output buffer.
  // Must be called from the audio thread.
  bool ProcessAudio(float* interleaved_output, size_t num_channels, size_t num_frames);

  // Listener
  void SetListenerTransform(const Vec3& pos, const Quat& rot);

  // Sources
  ResonanceAudioApi::SourceId CreateSoundObject(RenderingMode mode);
  ResonanceAudioApi::SourceId CreateAmbisonicSource(size_t num_channels);
  ResonanceAudioApi::SourceId CreateStereoSource(size_t num_channels);
  void DestroySource(ResonanceAudioApi::SourceId id);
  void SetSourceTransform(ResonanceAudioApi::SourceId id, const Vec3& pos, const Quat& rot);
  void SetSourceVolume(ResonanceAudioApi::SourceId id, float volume);
  void SetSourceInterleavedBuffer(ResonanceAudioApi::SourceId id, const float* data, size_t num_channels, size_t num_frames);

  // Rooms
  void SetRoomProperties(const vraudio::RoomProperties& props, const float* rt60s /* 9 values */);

 private:
  Engine();
  ~Engine();

  // PIMPL-style: hides resonance internals.
  ResonanceAudioApi* api_ = nullptr;
  // ... lockless task queue or other members ...
};

} // namespace audioengine
```

Suggested implementation sketch: `src/Engine.cpp`

```cpp
// Minimal engine wrapper implementation (outline)
#include "Engine.h"
#include "api/resonance_audio_api.h"
#include "utils/lockless_task_queue.h" // for posting setters

namespace audioengine {

Engine* Engine::Create(size_t num_output_channels, size_t frames_per_buffer, int sample_rate_hz) {
  Engine* e = new Engine();
  e->api_ = vraudio::CreateResonanceAudioApi(num_output_channels, frames_per_buffer, sample_rate_hz);
  if (!e->api_) { delete e; return nullptr; }
  // initialize other internals (task queue, conversion buffers)
  return e;
}

void Engine::Destroy(Engine* engine) {
  if (!engine) return;
  delete engine->api_; // matches API doc: caller owns pointer
  delete engine;
}

bool Engine::ProcessAudio(float* interleaved_output, size_t num_channels, size_t num_frames) {
  // Example: call into ResonanceAudioApi directly. If setters were posted
  // to a task queue, ensure they were executed on audio-thread before this.
  return api_->FillInterleavedOutputBuffer(num_channels, num_frames, interleaved_output);
}

void Engine::SetListenerTransform(const Vec3& pos, const Quat& rot) {
  // Post to task queue or call directly if safe. Example: post a lambda
  // to be executed on audio thread that calls SetHeadPosition/Rotation.
  api_->SetHeadPosition(pos.x, pos.y, pos.z);
  api_->SetHeadRotation(rot.x, rot.y, rot.z, rot.w);
}

ResonanceAudioApi::SourceId Engine::CreateSoundObject(RenderingMode mode) {
  return api_->CreateSoundObjectSource(mode);
}

void Engine::SetSourceInterleavedBuffer(ResonanceAudioApi::SourceId id, const float* data, size_t num_channels, size_t num_frames) {
  // This should be called from audio thread (or ensure api_->SetInterleavedBuffer is safe).
  api_->SetInterleavedBuffer(id, data, num_channels, num_frames);
}

void Engine::SetRoomProperties(const vraudio::RoomProperties& props, const float* rt60s) {
  // Convert RoomProperties -> ReflectionProperties / ReverbProperties and call API setters.
  vraudio::ReflectionProperties rp;
  // ... populate rp from props ...
  api_->SetReflectionProperties(rp);
  vraudio::ReverbProperties rv;
  for (int i = 0; i < 9; ++i) rv.rt60_values[i] = rt60s[i];
  api_->SetReverbProperties(rv);
}

} // namespace audioengine
```

Notes & guidance:
- The wrapper should enforce that `ProcessAudio` is called from the audio thread. Setters may be either posted to an internal queue (recommended) or made lock-free.
- Offer convenience overloads for interleaved vs planar buffers; the repo includes `utils/planar_interleaved_conversion.h` to help.
- Provide small helpers that convert `RoomProperties` → `ReflectionProperties`/`ReverbProperties`.

Example usage:

```cpp
auto engine = audioengine::Engine::Create(2, 512, 48000);
audioengine::Vec3 pos{0,0,0}; audioengine::Quat rot{0,0,0,1};
engine->SetListenerTransform(pos, rot);
auto src = engine->CreateSoundObject(vraudio::kBinauralHighQuality);
// feed buffers on audio thread
// engine->ProcessAudio(output, 2, 512);
audioengine::Engine::Destroy(engine);
```

---

If you want, I can add these files to the repo (header + implementation) as a starting point. I can also wire in `lockless_task_queue` usage and conversion helpers so the wrapper is production-ready.

---

## Exhaustive per-header one-line summaries (resonance-audio)

Note: paths are relative to repository root.

### api/
- `resonance-audio/resonance_audio/api/resonance_audio_api.h` — Public C-compatible audio API: engine factory, source lifecycle, listener and room setters, buffer IO.
- `resonance-audio/resonance_audio/api/resonance_c_api.h` — C wrapper types and handle definitions for the public API.
- `resonance-audio/resonance_audio/api/binaural_surround_renderer.h` — Binaural surround renderer interface and factory for surround decoding.

### graph/
- `resonance-audio/resonance_audio/graph/graph_manager.h` — Manages construction and processing of audio node graph and sources.
- `resonance-audio/resonance_audio/graph/graph_manager_config.h` — Config constants for `GraphManager`.
- `resonance-audio/resonance_audio/graph/resonance_audio_api_impl.h` — Concrete implementation of the `ResonanceAudioApi` interface.
- `resonance-audio/resonance_audio/graph/system_settings.h` — Global runtime settings (head transform, reverb/reflection props, source params).
- `resonance-audio/resonance_audio/graph/source_parameters_manager.h` — Stores and processes per-source parameters (position, volume, occlusion, etc.).
- `resonance-audio/resonance_audio/graph/source_graph_config.h` — Source graph config constants.
- `resonance-audio/resonance_audio/graph/ambisonic_binaural_decoder_node.h` — Node that decodes ambisonic buffers to binaural.
- `resonance-audio/resonance_audio/graph/ambisonic_mixing_encoder_node.h` — Ambisonic mixing/encoding node.
- `resonance-audio/resonance_audio/graph/ambisonic_mixing_encoder_node.h` — (duplicate) ambisonic mixing encoder.
- `resonance-audio/resonance_audio/graph/hoa_rotator_node.h` — Higher-order ambisonic rotator node.
- `resonance-audio/resonance_audio/graph/foa_rotator_node.h` — FOA rotator node.
- `resonance-audio/resonance_audio/graph/ambisonic_binaural_decoder_node.h` — Node wrapping ambisonic binaural decoder.
- `resonance-audio/resonance_audio/graph/ambisonic_binaural_decoder_node.h` — (duplicate) ambisonic binaural decode node.
- `resonance-audio/resonance_audio/graph/reverb_node.h` — Spectral reverb node.
- `resonance-audio/resonance_audio/graph/reflections_node.h` — Early reflections node; encodes reflections into ambisonics.
- `resonance-audio/resonance_audio/graph/occlusion_node.h` — Applies occlusion-based lowpass/gain to inputs.
- `resonance-audio/resonance_audio/graph/near_field_effect_node.h` — Near-field effect processing node.
- `resonance-audio/resonance_audio/graph/stereo_mixing_panner_node.h` — Stereo panner and mixer node.
- `resonance-audio/resonance_audio/graph/mixer_node.h` — Generic node wrapper for mixing outputs.
- `resonance-audio/resonance_audio/graph/gain_node.h` — Per-buffer gain node.
- `resonance-audio/resonance_audio/graph/gain_mixer_node.h` — Mixer combining gain-regulated channels.
- `resonance-audio/resonance_audio/graph/mono_from_soundfield_node.h` — Converts ambisonic/soundfield to mono mix.
- `resonance-audio/resonance_audio/graph/buffered_source_node.h` — Source node that reads from an `AudioBuffer`.

### node/
- `resonance-audio/resonance_audio/node/node.h` — Base node type definitions used by the graph.
- `resonance-audio/resonance_audio/node/processing_node.h` — Base class for processing nodes.
- `resonance-audio/resonance_audio/node/source_node.h` — Source node interface.
- `resonance-audio/resonance_audio/node/sink_node.h` — Sink node (output) interface.
- `resonance-audio/resonance_audio/node/publisher_node.h` — Publisher node type used in graph.
- `resonance-audio/resonance_audio/node/subscriber_node.h` — Subscriber node type.

### geometrical_acoustics/
- `resonance-audio/resonance_audio/geometrical_acoustics/scene_manager.h` — Builds Embree scenes and manages reflection-kernel associations.
- `resonance-audio/resonance_audio/geometrical_acoustics/mesh.h` — Mesh and triangle vertex definitions.
- `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_ray.h` — Ray primitive that wraps Embree RTCRay and stores per-band energies.
- `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_source.h` — Point-source sampling and stratified ray generation.
- `resonance-audio/resonance_audio/geometrical_acoustics/acoustic_listener.h` — Listener struct holding per-band IR energy responses.
- `resonance-audio/resonance_audio/geometrical_acoustics/path_tracer.h` — PathTracer interface for tracing rays.
- `resonance-audio/resonance_audio/geometrical_acoustics/path.h` — Path representation used for storing ray paths.
- `resonance-audio/resonance_audio/geometrical_acoustics/reflection_kernel.h` — Reflection kernels and sampling functions.
- `resonance-audio/resonance_audio/geometrical_acoustics/impulse_response_computer.h` — IR computation from traced paths.
- `resonance-audio/resonance_audio/geometrical_acoustics/proxy_room_estimator.h` — Estimates RT60/room parameters from samples.
- `resonance-audio/resonance_audio/geometrical_acoustics/estimating_rt60.h` — RT60 estimation helpers.
- `resonance-audio/resonance_audio/geometrical_acoustics/collection_kernel.h` — Kernel used for collecting path contributions.
- `resonance-audio/resonance_audio/geometrical_acoustics/sampling.h` — Sampling helpers for spheres and stratified sampling.
- `resonance-audio/resonance_audio/geometrical_acoustics/parallel_for.h` — Small parallel_for utility for CPU parallelism.
- `resonance-audio/resonance_audio/geometrical_acoustics/sphere.h` — Sphere primitive utilities.
- `resonance-audio/resonance_audio/geometrical_acoustics/test_util.h` — Test helpers for geometrical acoustics.

### base/
- `resonance-audio/resonance_audio/base/constants_and_types.h` — Fundamental constants and type aliases.
- `resonance-audio/resonance_audio/base/integral_types.h` — Integer type defs.
- `resonance-audio/resonance_audio/base/aligned_allocator.h` — Aligned memory allocator used by AudioBuffer.
- `resonance-audio/resonance_audio/base/channel_view.h` — ChannelView used to access planar channels.
- `resonance-audio/resonance_audio/base/audio_buffer.h` — Planar multi-channel AudioBuffer container.
- `resonance-audio/resonance_audio/base/source_parameters.h` — Per-source parameter struct definitions.
- `resonance-audio/resonance_audio/base/object_transform.h` — Transform utilities (Vec/Quat) and world transforms.
- `resonance-audio/resonance_audio/base/simd_utils.h` — SIMD helper utilities.
- `resonance-audio/resonance_audio/base/simd_macros.h` — SIMD macro definitions.
- `resonance-audio/resonance_audio/base/misc_math.h` — Math helpers used across code.
- `resonance-audio/resonance_audio/base/logging.h` — Lightweight logging macros.
- `resonance-audio/resonance_audio/base/unique_ptr_wrapper.h` — UniquePtr wrapper helper for C++ ABI edges.
- `resonance-audio/resonance_audio/base/spherical_angle.h` — Spherical angle utilities.

### dsp/
- `resonance-audio/resonance_audio/dsp/fft_manager.h` — FFT manager used by partitioned FFT filters and reverb.
- `resonance-audio/resonance_audio/dsp/partitioned_fft_filter.h` — Partitioned FFT filter for long convolutions.
- `resonance-audio/resonance_audio/dsp/spectral_reverb.h` — Spectral reverb implementation.
- `resonance-audio/resonance_audio/dsp/spectral_reverb_constants_and_tables.h` — Tables/constants for spectral reverb.
- `resonance-audio/resonance_audio/dsp/reverb_onset_compensator.h` — Onset compensation for spectral reverb.
- `resonance-audio/resonance_audio/dsp/reverb_onset_update_processor.h` — Reverb onset update helper.
- `resonance-audio/resonance_audio/dsp/sh_hrir_creator.h` — Creates spherical harmonic HRIRs used for ambisonic decoding.
- `resonance-audio/resonance_audio/dsp/partitioned_fft_filter.h` — (duplicate) partitioned FFT filter.
- `resonance-audio/resonance_audio/dsp/resampler.h` — Resampler utility for HRIR/sample rate conversions.
- `resonance-audio/resonance_audio/dsp/fft_manager.h` — (duplicate) FFT manager.
- `resonance-audio/resonance_audio/dsp/reverb_onset_compensator.h` — (duplicate) onset compensator.

More DSP headers:
- `resonance-audio/resonance_audio/dsp/reflections_processor.h` — Processes early reflection encoding into ambisonic buffers.
- `resonance-audio/resonance_audio/dsp/reflection.h` — Single reflection modeling utilities.
- `resonance-audio/resonance_audio/dsp/occlusion_calculator.h` — Computes occlusion attenuation / filtering parameters from geometry and directivity.
- `resonance-audio/resonance_audio/dsp/near_field_processor.h` — Near-field effect processors (gain/filters).
- `resonance-audio/resonance_audio/dsp/stereo_panner.h` — Stereo panning implementation.
- `resonance-audio/resonance_audio/dsp/mixer.h` — Low-level mixer utilities.
- `resonance-audio/resonance_audio/dsp/gain.h` — Simple gain utilities.
- `resonance-audio/resonance_audio/dsp/gain_processor.h` — Per-sample/per-buffer gain processor.
- `resonance-audio/resonance_audio/dsp/gain_mixer.h` — Mono/stereo gain mixer.
- `resonance-audio/resonance_audio/dsp/distance_attenuation.h` — Distance attenuation models (linear/log/none).
- `resonance-audio/resonance_audio/dsp/delay_filter.h` — Delay line and filters.
- `resonance-audio/resonance_audio/dsp/mono_pole_filter.h` — Simple mono-pole low-pass filter used for occlusion.
- `resonance-audio/resonance_audio/dsp/multi_channel_iir.h` — Multi-channel IIR filter helpers.
- `resonance-audio/resonance_audio/dsp/partitioned_fft_filter.h` — (mentioned) partitioned convolution helper.
- `resonance-audio/resonance_audio/dsp/utils.h` — DSP small helpers.
- `resonance-audio/resonance_audio/dsp/fft_manager.h` — (already listed) FFT manager.

### ambisonics/
- `resonance-audio/resonance_audio/ambisonics/ambisonic_codec.h` — Ambisonic codec API.
- `resonance-audio/resonance_audio/ambisonics/ambisonic_codec_impl.h` — Implementation of ambisonic encoding/decoding.
- `resonance-audio/resonance_audio/ambisonics/ambisonic_binaural_decoder.h` — Ambisonic → binaural convolution decoder.
- `resonance-audio/resonance_audio/ambisonics/ambisonic_lookup_table.h` — Lookup table for ambisonic encoding coefficients.
- `resonance-audio/resonance_audio/ambisonics/ambisonic_spread_coefficients.h` — Spread coefficients for ambisonic source spread modelling.
- `resonance-audio/resonance_audio/ambisonics/hoa_rotator.h` — High-order ambisonic rotator.
- `resonance-audio/resonance_audio/ambisonics/foa_rotator.h` — First-order ambisonic rotator.
- `resonance-audio/resonance_audio/ambisonics/associated_legendre_polynomials_generator.h` — Generator for spherical harmonic polynomials.
- `resonance-audio/resonance_audio/ambisonics/utils.h` — Ambisonic utility helpers.
- `resonance-audio/resonance_audio/ambisonics/stereo_from_soundfield_converter.h` — Converts ambisonic soundfield to stereo.

### utils/
- `resonance-audio/resonance_audio/utils/lockless_task_queue.h` — Lock-free task queue for posting tasks from main to audio thread.
- `resonance-audio/resonance_audio/utils/semi_lockless_fifo.h` — Semi-lockless FIFO utility.
- `resonance-audio/resonance_audio/utils/threadsafe_fifo.h` — Thread-safe FIFO for producer/consumer patterns.
- `resonance-audio/resonance_audio/utils/planar_interleaved_conversion.h` — Helpers to convert between planar and interleaved buffers.
- `resonance-audio/resonance_audio/utils/sample_type_conversion.h` — Float/int16 conversion helpers.
- `resonance-audio/resonance_audio/utils/wav.h` — WAV file definitions and helpers.
- `resonance-audio/resonance_audio/utils/wav_reader.h` — WAV reader helper.
- `resonance-audio/resonance_audio/utils/vorbis_stream_encoder.h` — Vorbis encoder utilities.
- `resonance-audio/resonance_audio/utils/ogg_vorbis_recorder.h` — Recorder helper writing Ogg Vorbis.
- `resonance-audio/resonance_audio/utils/buffer_partitioner.h` — Partitions buffers for partitioned-FFT processing.
- `resonance-audio/resonance_audio/utils/buffer_unpartitioner.h` — Reverse of partitioner.
- `resonance-audio/resonance_audio/utils/buffer_crossfader.h` — Crossfading buffers for smooth transitions.
- `resonance-audio/resonance_audio/utils/task_thread_pool.h` — Thread-pool helpers for worker tasks.
- `resonance-audio/resonance_audio/utils/test_util.h` — Unit test helpers.
- `resonance-audio/resonance_audio/utils/pseudoinverse.h` — Pseudoinverse math helper (used by ambisonics code).
- `resonance-audio/resonance_audio/utils/sum_and_difference_processor.h` — Sum/difference processors used in decoding.

### platform integrations & common
- `resonance-audio/resonance_audio/platforms/common/room_properties.h` — Platform-agnostic RoomProperties struct for host apps.
- `resonance-audio/resonance_audio/platforms/common/room_effects_utils.h` — Utilities for mapping room materials → reflection coefficients.
- `resonance-audio/resonance_audio/platforms/common/utils.h` — Platform helper utilities.
- `resonance-audio/resonance_audio/platforms/unity/unity.h` — Unity adapter header wrapping the Resonance API for Unity plugin usage.
- `resonance-audio/resonance_audio/platforms/unity/unity_reverb_computer.h` — Unity-specific reverb computation helper.
- `resonance-audio/resonance_audio/platforms/unity/unity_nativeaudioplugins.h` — Unity native audio plugin helpers.
- `resonance-audio/resonance_audio/platforms/fmod/fmod.h` — FMOD integration header.
- `resonance-audio/resonance_audio/platforms/wwise/*` — Wwise plugin headers (renderer, fx factory, attachments etc.).
- `resonance-audio/resonance_audio/platforms/vst/*` — VST plugin UI and host integration headers.

### third_party & generated
- `resonance-audio/third_party/SADIE_hrtf_database/generated/hrtf_assets.h` — Generated HRTF asset table for binaural rendering.

### config/
- `resonance-audio/resonance_audio/config/global_config.h` — Global compile/runtime config constants.
- `resonance-audio/resonance_audio/config/source_config.h` — Default/source configuration constants.

### other base / misc headers
- `resonance-audio/resonance_audio/dsp/filter_coefficient_generators.h` — IIR/FIR coefficient generator helpers.
- `resonance-audio/resonance_audio/dsp/fir_filter.h` — FIR filter helper.
- `resonance-audio/resonance_audio/dsp/circular_buffer.h` — Circular buffer helper for streaming.
- `resonance-audio/resonance_audio/base/channel_view.h` — Channel view wrapper for planar buffers.
- `resonance-audio/resonance_audio/base/unique_ptr_wrapper.h` — UniquePtr wrapper (ABI helper).

---

If you want the file to include inline brief prototypes (signatures) for each header, I can expand each one-line entry to include the most relevant classes/functions from that header. This will be significantly longer but can be generated automatically.
