Awesome—here’s the concrete, host-side spec distilled from your consolidated headers. Treat this as the contract for your **audio game engine host** (engine runtime) to make the Resonance-style DSP core fully usable.

---

# 🎮 Host Implementation Spec (from the headers you provided)

## 0) Goals & Boundaries

* **DSP core provides:** sample processing (`ProcessSource`, `ProcessListener`), per-source/ listener params, channel conversion & gain utilities, optional reverb precompute.
* **Host must provide:** device I/O, threading/scheduling, buffers, scene/ECS sync, parameter transport, format negotiation, asset/streaming, diagnostics, error handling.

---

## 1) Lifecycle & Configuration

### Required host choices (once, before audio starts)

* `sample_rate_device` (e.g., 48k) and `frames_per_buffer` (e.g., 128/256/512).
* `output_layout_device` (e.g., stereo interleaved).
* `engine_layout_internal` (e.g., stereo or ambisonics) and whether to render HRTF/headphone mode.

### Host calls

1. `api = CreateResonanceAudioApi(sample_rate_device, frames_per_buffer);`
2. Optional: set global listener options

   * `SetListenerGain(float)`
   * `SetListenerStereoSpeakerMode(bool)`
3. Start audio backend (WASAPI/CoreAudio/ALSA/etc.), open stream matching your device format.

### Host responsibilities

* **Own** the `api` handle and destroy it on shutdown.
* Recreate/retune on device change (sample rate/buffer size change).

---

## 2) Audio I/O & Processing Loop

### RT (audio) thread contract

* **Never block**: no allocations, no locks, no logging with syscalls in the callback.
* Pull parameters via **lock-free snapshot** set by the main thread.
* Process **exactly `length` frames** provided by the backend.

### Canonical callback flow (interleaved device I/O)

```
onAudioCallback(outbuffer, length, outchannels):
  // 1) Consume pending RT-safe commands (param snapshots, source starts/stops)
  applyParamSnapshot()

  // 2) For each active source with input audio this block:
  for each Source s:
     // Input may be decoded off-thread → ringbuffer → RT read
     float* srcInterleaved = s.readChunk(length)
     if srcInterleaved:
        // If engine expects planar per-source input, convert here (host-side)
        deinterleaveOrWrap(srcInterleaved, tmpPlanar)
        ProcessSource(s.id, s.numChannels, length, tmpPlanarOrInterleavedPtr)

  // 3) Render listener mix into host output buffer
  ProcessListener(length, outbuffer) // expected interleaved float*, device format

  return OK
```

### Format mediation you must handle

* If device layout ≠ engine internal, use `ChannelConverter` before/after engine calls (upmix/downmix).
* If sample rate differs (e.g., 44.1k content → 48k device), resample **content** off the RT thread; the **engine** is driven at the device rate.
* If engine’s `ProcessSource` expects planar, deinterleave before calling; if it accepts interleaved, pass straight through.

---

## 3) Source & Listener Lifecycle

### Host-side ownership

* The host owns **Source objects** and their input queues.
* Map ECS entities → `SourceId` (opaque).

### Required flows

* **Create**: `id = CreateSoundObject(...)` or `CreateSoundfield(...)`.
* **Destroy**: `DestroySource(id)`.
* **Push audio** each block: `ProcessSource(id, num_channels, num_frames, float* input)`.
* **Spatial updates** (each frame or when changed):
  `SetSourceTransform(id, px,py,pz, qx,qy,qz,qw)`

### Parameter transport (RT-safe)

* Main thread writes to a **double-buffered parameter store** (POD structs / atomics).
* RT thread reads fresh snapshot at block start.
* Supported params from your headers: gain, spread, directivity (alpha/order), near-field, occlusion intensity, room send, distance attenuation. Provide **per-source smoothing** (linear/exponential) to avoid zipper noise.

---

## 4) Assets, Decoding, and Streaming

### Host delivers audio to the engine

* Decode compressed assets (OGG/Opus/etc.) **off the RT thread** into a per-source ringbuffer in **engine sample rate**.
* For short SFX, pre-decode into memory.
* For long streams (music/VO), page in chunks and keep \~2–4 blocks of headroom.

### Buffering rules

* **No heap** in RT path → use fixed ringbuffers or mem-pools.
* Interleaved storage is fine for disk/decoder; convert to planar only if the API requires.

---

## 5) Channel Conversion & Gain

* Use `Gain::ApplyGain*` as the last step on per-source or pre-mix automation where needed.
* Use `ChannelConverter` for:

  * Content channel layout → engine expected input layout.
  * Engine render output → device layout (if the engine does not already render in device layout).

Keep conversion **out of RT** when possible (e.g., static upmix for mono SFX cached), but runtime conversion is acceptable if necessary and cheap.

---

## 6) Scene Acoustics (Reverb Computer)

### Heavy, off-thread precompute

* Build geometry once (or on level load):
  `InitializeReverbComputer(num_vertices, num_triangles, vertices, triangles, material_indices, scattering)`
* Compute room params:
  `ComputeRt60sAndProxyRoom(...) → output_rt60s + output_proxy_room`
* Feed into runtime reverb:
  `SetRoomProperties(output_proxy_room, output_rt60s)` (host must provide/bridge a setter; your summary indicates such a setter exists in the broader code).

### Host duties

* Ensure vertex format (xyz float) and triangle indices (0-based) match expectations.
* Manage lifetime of uploaded geometry and materials.
* Allow progressive/batched runs so the game stays responsive.

---

## 7) Threading Model

* **Audio thread**: device callback; drives `ProcessSource`/`ProcessListener`; reads param snapshots; no blocking.
* **Decode/stream thread(s)**: file I/O, codec decode, resampling, pre-conversion, fill ringbuffers.
* **Main/game thread**: ECS, transforms, parameter updates, source create/destroy, reverb compute orchestration.

**Cross-thread bridges**

* Lock-free SPSC/MPSC queues for:

  * Commands (create/destroy, state changes) → RT.
  * Metrics (underruns, CPU %) → main.
* Double-buffered structs for:

  * Listener pose, global options.
  * Per-source params (gain/spread/etc.).

---

## 8) Error Handling & Resilience

* Define host-visible states: `Running`, `Starved`, `DeviceLost`, `Reconfiguring`.
* On **underrun**: count + expose; optionally duck mix complexity (disable expensive effects) next frames.
* On **device loss**: stop stream → re-open → recreate API if rate/buffer changed.
* Validate inputs to `ProcessSource` (null/size/channel count); sanitize out-of-range params.

---

## 9) Diagnostics & Tooling

* **Metrics (per second)**: callback duration (avg/max), XRuns, buffer fill % per source, voices active, resampler cost, late commands.
* **Debug audio taps**: optional record of pre-mix/post-mix (ringbuffer to a file writer thread).
* **Parameter tracing**: frame index + param snapshots (not from RT directly—queue a compact event).

---

## 10) Host API Surface (what your engine should expose upward)

Minimal C++-style host façade (engine-agnostic):

```cpp
struct HostInit {
  int sample_rate;
  int frames_per_buffer;
  int device_out_channels; // e.g., 2
};

struct SourceParams {
  float gain;      // linear
  float spread;    // degrees
  float directivity_alpha;
  float directivity_order;
  float near_field_gain;
  float occlusion; // 0..1
  float room_send; // 0..1
};

class AudioHost {
public:
  bool start(const HostInit&);
  void stop();

  // Listener
  void setListenerPose(float px, float py, float pz,
                       float qx, float qy, float qz, float qw);
  void setListenerGain(float gain);
  void setStereoSpeakerMode(bool enabled);

  // Sources
  uint32_t createSoundObject(int channels /*1 or 2*/);
  uint32_t createSoundfield(int ambiChannels /*ambiX*/);
  void     destroySource(uint32_t id);

  // Params (async-safe: enqueue or write to double buffer)
  void setSourceParams(uint32_t id, const SourceParams&);
  void setSourcePose(uint32_t id, /*pos+quat*/);

  // Audio supply (non-RT threads push PCM into per-source ringbuffers)
  bool pushPcm(uint32_t id, const float* interleaved, int frames, int channels);
};
```

Inside, `AudioHost` owns:

* `ResonanceAudioApi* api`
* Device backend instance
* Source table (id → ringbuffer, params, format)
* Converters (prebuilt upmix/downmix paths if needed)
* Queues/snapshots for RT handoff

---

## 11) Minimal Audio Callback (C++ pseudocode)

```cpp
UNITY_AUDIODSP_RESULT RendererProcessCallback(...,
    float* inbuffer, float* outbuffer,
    unsigned int length, int inchannels, int outchannels) {

  RTSectionGuard g; // no-alloc, just scope timing if needed

  host.applyParamSnapshotRT();

  // Feed per-source chunks into the engine
  for (auto& s : host.activeSourcesRT()) {
    float* src = s.ring.readInterleaved(length);
    if (src) {
      float* srcPlanar = host.tmpPool.acquirePlanar(s.channels, length); // preallocated
      deinterleave(src, srcPlanar, s.channels, length);
      ProcessSource(s.id, s.channels, length, srcPlanar);
    }
  }

  // Render final mix directly into device buffer
  ProcessListener(length, outbuffer);

  return UNITY_AUDIODSP_OK;
}
```

---

## 12) Rust FFI outline (if your host is Rust + C++ DSP)

```rust
#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        type ResonanceAudioApi;

        fn CreateResonanceAudioApi(sr: i32, fpb: i32) -> *mut ResonanceAudioApi;
        fn DestroyResonanceAudioApi(api: *mut ResonanceAudioApi);

        type SourceId = u32;

        fn CreateSoundObject(api: *mut ResonanceAudioApi, rendering_mode: i32) -> SourceId;
        fn CreateSoundfield(api: *mut ResonanceAudioApi, num_channels: i32) -> SourceId;
        fn DestroySource(api: *mut ResonanceAudioApi, id: SourceId);

        fn ProcessSource(api: *mut ResonanceAudioApi, id: SourceId,
                         num_channels: usize, num_frames: usize, input: *const f32);
        fn ProcessListener(api: *mut ResonanceAudioApi, num_frames: usize, out: *mut f32);

        fn SetSourceTransform(api: *mut ResonanceAudioApi, id: SourceId,
                              px:f32,py:f32,pz:f32, qx:f32,qy:f32,qz:f32,qw:f32);
        fn SetSourceGain(api: *mut ResonanceAudioApi, id: SourceId, gain: f32);
        // ... other setters
    }
}
// RT path uses fixed-capacity ringbuffers (e.g., lockfree mpmc), no alloc in callback.
```

---

## 13) Acceptance Checklist (ship-ready)

* [ ] No lock/alloc in RT callback (verified with allocator guards).
* [ ] Device change (rate/buffer) triggers clean reconfig w/o crash.
* [ ] 500+ source create/destroy cycles leak-free.
* [ ] Underrun resilience: graceful audio under CPU spike (metric increments, no stall).
* [ ] Param smoothing prevents clicks at 60 Hz update.
* [ ] Channel conversion correctness (mono↔stereo, ambi if used).
* [ ] Reverb computer runs off-thread and updates room safely at block boundaries.
* [ ] Record/playback harness reproduces a frame-accurate mix for regression tests.

---

## 14) Common Pitfalls to Avoid

* Passing **wrong channel layout** (engine expects planar vs device interleaved).
* Updating transforms directly from game thread → use snapshot; avoid locks.
* Feeding `ProcessSource` with frames ≠ callback `length`.
* Running reverb compute or decoding on the RT thread.
* Forgetting to smooth gain/spread → zipper noise.

---

If you want, I can turn this into a tiny **reference host** (C++ or Rust): one source, one listener, WAV file streaming, with a WASAPI/CoreAudio backend and the RT-safe queues wired up.

