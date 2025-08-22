
# 🎧 Resonance Audio — Consolidated Header Reference (all provided headers)

> Contains original declarations and their roles from these headers:
>
> * `base/audio_buffer.h`
> * `dsp/gain.h`
> * `dsp/channel_converter.h`
> * `api/resonance_audio_api.h`
> * `platforms/unity/unity_nativeaudioplugins.h`
> * `platforms/unity/unity_reverb_computer.h`

---

## 1 — `base/audio_buffer.h`

**Class**: `AudioBuffer`
**Typical Fields / Methods** (as seen in headers / patterns):

```cpp
class AudioBuffer {
 public:
  AudioBuffer(int num_channels, int num_frames);
  int num_channels() const;
  int num_frames() const;
  float* channel(int channel_index);
  const float* channel(int channel_index) const;
  int size() const;

 private:
  int num_channels_;
  int num_frames_;
  float* channel_data_[kMaxChannels];
};
```

**Role:** canonical container for audio samples; used for passing audio between DSP units (planar or interleaved access variants exist in the codebase).

---

## 2 — `dsp/gain.h`

**Class**: `Gain`

**Representative Methods:**

```cpp
class Gain {
 public:
  static void ApplyGain(const AudioBuffer& input, float gain, AudioBuffer* output);
  static void ApplyGainInterleaved(float* buffer, int num_frames, int num_channels, float gain);
};
```

**Role:** scalar amplitude adjustment for buffers (mixing, fades, level normalization). Fast, frequently invoked utility.

---

## 3 — `dsp/channel_converter.h`

**Class**: `ChannelConverter`

**Representative Methods:**

```cpp
class ChannelConverter {
 public:
  void Convert(const AudioBuffer& input, AudioBuffer* output);
  void Upmix(const AudioBuffer& mono, AudioBuffer* stereo);
  void Downmix(const AudioBuffer& stereo, AudioBuffer* mono);
};
```

**Role:** converts between channel layouts (mono/stereo/multi-channel), responsible for upmix/downmix logic and preserving perceptual balance.

---

## 4 — `api/resonance_audio_api.h`

**Opaque Type / Struct:**

```cpp
struct ResonanceAudioApi;  // opaque handle type
```

**Key C API Functions (typical signatures from header patterns):**

```cpp
ResonanceAudioApi* CreateResonanceAudioApi(int sample_rate, int frames_per_buffer);
void DestroyResonanceAudioApi(ResonanceAudioApi* api);

using SourceId = /* alias / typedef inside header (opaque handle) */;

ResonanceAudioApi::SourceId CreateSoundfield(int num_channels);
ResonanceAudioApi::SourceId CreateSoundObject(RenderingMode rendering_mode);
void DestroySource(ResonanceAudioApi::SourceId id);

void ProcessSource(ResonanceAudioApi::SourceId id, size_t num_channels, size_t num_frames, float* input);
void ProcessListener(size_t num_frames, float* output);

void SetSourceTransform(ResonanceAudioApi::SourceId id, float px, float py, float pz,
                        float qx, float qy, float qz, float qw);
void SetListenerTransform(float px, float py, float pz, float qx, float qy, float qz, float qw);

void SetSourceGain(ResonanceAudioApi::SourceId id, float gain);
void SetSourceSpread(ResonanceAudioApi::SourceId id, float spread_deg);
void SetSourceDirectivity(ResonanceAudioApi::SourceId id, float alpha, float order);
void SetSourceListenerDirectivity(ResonanceAudioApi::SourceId id, float alpha, float order);
void SetSourceNearFieldEffectGain(ResonanceAudioApi::SourceId id, float near_field_effect_gain);
void SetSourceOcclusionIntensity(ResonanceAudioApi::SourceId id, float intensity);
void SetSourceRoomEffectsGain(ResonanceAudioApi::SourceId id, float room_effects_gain);
void SetSourceDistanceAttenuation(ResonanceAudioApi::SourceId id, float distance_attenuation);

// Listener controls:
void SetListenerGain(float gain);
void SetListenerStereoSpeakerMode(bool enable_stereo_speaker_mode);
```

**Role:** public engine boundary — creation/destruction, source lifecycle and parameter control, buffer I/O entry points (ProcessSource, ProcessListener), listener state.

---

## 5 — `platforms/unity/unity_nativeaudioplugins.h`

### Android 64-bit typedef fix (compatibility)

```cpp
#if defined(__ANDROID__) && defined(__LP64__)
#include "base/integral_types.h"
typedef int32  SInt32;
typedef uint32 UInt32;
typedef int64  SInt64;
typedef uint64 UInt64;
#endif
```

### Unity Plugin Callbacks (original names preserved)

**Renderer callbacks:**

```cpp
UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
RendererCreateCallback(UnityAudioEffectState* state);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
RendererReleaseCallback(UnityAudioEffectState* state);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
RendererProcessCallback(UnityAudioEffectState* state,
                        float* inbuffer, float* outbuffer,
                        unsigned int length, int inchannels, int outchannels);
```

**Spatializer callbacks:**

```cpp
UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
SpatializerCreateCallback(UnityAudioEffectState* state);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
SpatializerReleaseCallback(UnityAudioEffectState* state);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
SpatializerDistanceAttenuationCallback(UnityAudioEffectState* state,
                                       float distance_in, float attenuation_in,
                                       float* attenuation_out);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
SpatializerProcessCallback(UnityAudioEffectState* state, float* inbuffer, float* outbuffer,
                           unsigned int length, int inchannels, int outchannels);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
SpatializerSetFloatParameterCallback(UnityAudioEffectState* state, int index, float value);

UNITY_AUDIODSP_RESULT UNITY_AUDIODSP_CALLBACK
SpatializerGetFloatParameterCallback(UnityAudioEffectState* state, int index,
                                     float* value, char* valuestr);
```

**Plugin registration (C ABI):**

```cpp
extern "C" {
  int EXPORT_API
  UnityGetAudioEffectDefinitions(UnityAudioEffectDefinition*** definitionptr);
}
```

**Role:** This header defines the **host-integration entry points** — lifecycle and process callbacks used by Unity's Native Audio Plugin interface to forward audio buffers and parameter updates into the engine.

---

## 6 — `platforms/unity/unity_reverb_computer.h`

**C ABI functions (original names preserved):**

```cpp
extern "C" {

// Initializes the scene and data for ray tracing.
void EXPORT_API InitializeReverbComputer(int num_vertices, int num_triangles,
                                         float* vertices, int* triangles,
                                         int* material_indices,
                                         float scattering_coefficient);

// Computes RT60s and proxy room using ray tracing.
bool EXPORT_API ComputeRt60sAndProxyRoom(
    int total_num_paths, int num_paths_per_batch, int max_depth,
    float energy_threshold, float sample_position[3],
    float listener_sphere_radius, float sampling_rate,
    int impulse_response_num_samples, float* output_rt60s,
    RoomProperties* output_proxy_room);

}  // extern C
```

**Related types referenced:**

* `RoomProperties` (from `platforms/common/room_properties.h`) — structure containing room acoustic parameters (dimensions, absorption coefficients per band, etc.).
* `RoomProperties* output_proxy_room` is populated by `ComputeRt60sAndProxyRoom`.

**Role:** offline/heavy computation module — builds ray-traced acoustic estimates from scene geometry:

* `InitializeReverbComputer()` uploads triangle mesh (vertices, triangles) and material indices plus a global scattering coefficient.
* `ComputeRt60sAndProxyRoom()` runs stochastic ray tracing to estimate per-band RT60 values and infer a `RoomProperties` proxy suitable for runtime reverb rendering.

**Important parameter notes (from header):**

* `vertices` is an array of floats, 3 floats per vertex: `{v1_x, v1_y, v1_z, v2_x, v2_y, v2_z, ...}`.
* `triangles` is an array of indices, 3 indices per triangle: e.g. `{i0, i1, i2, ...}`.
* `material_indices` maps each triangle to a material index.
* `total_num_paths`, `num_paths_per_batch`, `max_depth`, `energy_threshold` control tracer quality/performance.
* `sample_position[3]` is the listener/sample origin.
* `listener_sphere_radius` denotes sampling aperture around sample position.
* `impulse_response_num_samples` and `sampling_rate` define IR discretization used for decay estimation.
* `output_rt60s` must be sized to the engine’s expected frequency band count.

---

## Cross-Header Integration / Observed Patterns

### 1. **Canonical Data Flow**

* **Source input** → `ProcessSource(...)` (push raw source buffers into engine).
* **Per-source processing** (spatialization, directivity, occlusion, near-field effects).
* **Mixing / Renderer** → `ProcessListener(...)` (pull final mixed output into host buffer).
* **Platform host** (Unity in headers) calls plugin callbacks (`RendererProcessCallback`, `SpatializerProcessCallback`) to feed buffers to/from the engine.

### 2. **Parameter & Control Set**

* Per-source parameter setters (gain, spread, directivity, occlusion, room send, near-field gain, transform).
* Listener control functions (`SetListenerTransform`, `SetListenerGain`, `SetListenerStereoSpeakerMode`).
* Room control via `SetRoomProperties(RoomProperties* room_properties, float* rt60s)` (from `unity.h` earlier) fed by `unity_reverb_computer.h` outputs.

### 3. **Separation of Concerns**

* **DSP primitives** (e.g., `Gain`, `ChannelConverter`) are small, reusable building blocks.
* **Engine API** (`ResonanceAudioApi`) exposes lifecycle and high-level operations.
* **Platform glue** (`unity_nativeaudioplugins.h`) maps host lifecycle/parameters to engine API calls.
* **Precomputation** (`unity_reverb_computer.h`) derives room-level acoustic parameters from scene geometry and materials.

### 4. **Threading & Real-time Rules (implicit)**

* `ProcessListener` and `ProcessSource` must be called from the audio thread — must be RT-safe (no blocking, heap-heavy allocations, or locks that can stall audio).
* `ComputeRt60sAndProxyRoom` is heavy and intended to run off the audio thread (batch parameter exists to allow cooperative processing).

---

## Usage / Integration Flow (engine-side summary)

1. **Initialization**

   * Initialize engine via `CreateResonanceAudioApi(...)` (or equivalent initialization functions in the headers).
   * Host configures sample rate / buffer sizes and optionally calls `Initialize(...)` (Unity-specific).

2. **Scene Acoustic Precomputation (optional)**

   * Call `InitializeReverbComputer(...)` with scene vertices/triangles/material indices.
   * Run `ComputeRt60sAndProxyRoom(...)` to get `output_rt60s` and `output_proxy_room`.
   * Pass results into engine via `SetRoomProperties(...)`.

3. **Source Lifecycle**

   * Create sources: `CreateSoundfield(...)` (ambiX) or `CreateSoundObject(...)` (positional).
   * Push per-source audio via `ProcessSource(...)`.
   * Update per-source transforms/parameters with `SetSourceTransform`, `SetSourceGain`, etc.
   * Destroy sources with `DestroySource(...)` when done.

4. **Rendering Loop (audio thread)**

   * Host audio callback invokes `ProcessListener(num_frames, output)` to retrieve final mixed output (interleaved float buffer).
   * Ensure parameter updates from main thread are synchronized in an RT-safe way (double-buffered snapshots or lock-free atomics).

5. **Platform Binding**

   * Host (Unity) registers plugin via `UnityGetAudioEffectDefinitions(...)`.
   * Unity invokes `RendererCreateCallback`, `RendererProcessCallback`, `SpatializerCreateCallback`, `SpatializerProcessCallback`, etc., which should call into the engine API functions.

---

## Noted Implementation Details / Caveats (from headers)

* `SetSourceTransform` comment referenced scale originally but function parameters only include position + quaternion (px,py,pz,qx,qy,qz,qw) — watch for stale comments or additional overloads in implementation files.
* `output_rt60s` sizing and frequency band definition must match whatever `RoomProperties` expects (band count & ordering).
* `triangles` index base (0-based vs 1-based) is not explicitly documented — typical C++ practice is 0-based; confirm with implementation if available.
* `unity_android.lds` must include exported symbols for Android packaging (platform build detail).
