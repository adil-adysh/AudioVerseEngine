# Audio‑Only Game Engine — **Design.md** (Rust, Android + Windows)
**Backends:** Android (AAudio via Oboe), Windows (WASAPI via CPAL)  
**Spatialization:** Resonance Audio SDK (FFI).  
**Focus:** Audio‑first games (no mandatory graphics), ultra‑low latency, sample‑accurate events, robust room acoustics.

---

## 0) Goals & Non‑Goals
**Goals**
- < 10 ms end‑to‑end output latency on capable Android devices; stable low latency on Windows (WASAPI Exclusive when available).
- Sample‑accurate scheduling and parameter automation.
- Binaural/HRTF spatialization with early reflections & late reverb.
- Data‑driven scenes; accessible interaction (TTS friendly pipeline, ducking, dynamic range presets).
- One portable Rust core; thin platform backends.

**Non‑Goals**
- Rich 3D graphics/editor for v1; we support JSON/RON/YAML content and CLI tooling first.

---

## 1) Architectural Overview
```
+---------------------------------------------------------------+
|                           GAME LAYER                          |
|  ECS (hecs) | Systems (Rooms, Occlusion, Sequencer) | Scripting|
+---------------------------------------------------------------+
|                        AUDIO SUBSYSTEM                        |
|  Command Bus (lock‑free)  |  Scheduler (frame‑time)          |
|  Sources (clips/streams/TTS)  |  Buses (Master/Music/SFX/VO)  |
|  Spatialization: ResonanceAudioApi (FFI wrapper)              |
+---------------------------------------------------------------+
|                     AUDIO DEVICE BACKENDS                     |
|  Android: Oboe/AAudio (oboe‑rs)  |  Windows: CPAL (WASAPI)    |
+---------------------------------------------------------------+
|                      PLATFORM & UTILITIES                     |
|  Asset IO, Decoders, Ring Buffers, Job System, Telemetry      |
+---------------------------------------------------------------+
```

**Key decision:** We integrate the **high‑level `vraudio::ResonanceAudioApi`** (thread‑safe, non‑blocking) rather than wiring internal graph nodes directly. This lets the engine stay simple, portable, and RT‑safe while still exposing room/spatial features.

---

## 2) Threading & Timing Model
- **Audio callback thread (platform)**: created by Oboe/CPAL; calls our closure with `num_frames`. RT‑safe: **no locks, no heap allocs, no logging**.
- **Audio engine thread (optional)**: preps/resamples/decodes, fills per‑source ring buffers; may be merged with game thread on desktop.
- **Game thread**: ECS update, scene logic, room detection, input → emits audio commands.
- **Clock**: device sample clock is truth. All scheduling uses **frame timestamps** relative to stream start. Components convert wall‑time to frames per block.

---

## 3) Rust Crates & Modules
```
crates/
  audio-backend/        # trait + Oboe + CPAL impls
  resonance-ffi/        # unsafe FFI bindings to ResonanceAudioApi
  resonance/            # safe wrapper (RA facade used by engine)
  engine-core/          # ECS, scheduler, buses, content, commands
  content-pipeline/     # packer, loudness scan, validation
  app-android/          # JNI shim, AAR packaging
  app-windows/          # binary launcher
```

**Primary dependencies**
- `oboe` (Android), `cpal` (Windows), `dasp` (DSP utils), `crossbeam` (queues), `ringbuf`, `serde` + `serde_json`/`ron`, `hecs` (ECS).  
- Build: `bindgen` (to generate `resonance-ffi` from headers) or maintain a curated C header + minimal C++ shim.

---

## 4) Audio Backend Abstraction (Rust)
```rust
pub trait AudioBackend: Send {
    fn start(&mut self, render: RenderFn) -> Result<(), BackendError>;
    fn stop(&mut self);
    fn sample_rate(&self) -> u32;
    fn buffer_size(&self) -> usize;    // frames per callback
    fn channels(&self) -> u16 { 2 }
}

pub type RenderFn = Arc<dyn Fn(&mut [f32], u32 /*sr*/, usize /*frames*/)
                              + Send + Sync + 'static>;
```
**Android** → `OboeBackend`: configures `PerformanceMode::LowLatency`, `SharingMode::Exclusive` if available.  
**Windows** → `CpalBackend`: WASAPI; prefer exclusive, fallback to shared.

---

## 5) Resonance Audio Integration (FFI & Safe Wrapper)
We wrap **`vraudio::ResonanceAudioApi`**. Public engine code never touches raw pointers.

### 5.1 Unsafe FFI surface (excerpt)
```rust
// resonance-ffi
pub enum ResonanceAudioApi {}
extern "C" {
  pub fn CreateResonanceAudioApi(
    num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32
  ) -> *mut ResonanceAudioApi;
  pub fn Destroy(api: *mut ResonanceAudioApi);
  pub fn FillInterleavedOutputBufferF32(api:*mut ResonanceAudioApi,
    num_channels: usize, num_frames: usize, buffer_ptr: *mut f32) -> bool;
  pub fn SetHeadPosition(api:*mut ResonanceAudioApi, x:f32,y:f32,z:f32);
  pub fn SetHeadRotation(api:*mut ResonanceAudioApi, x:f32,y:f32,z:f32,w:f32);
  pub fn SetMasterVolume(api:*mut ResonanceAudioApi, vol:f32);
  pub fn SetStereoSpeakerMode(api:*mut ResonanceAudioApi, enabled: bool);
  pub fn CreateSoundObjectSource(api:*mut ResonanceAudioApi, mode:i32) -> u32;
  pub fn CreateAmbisonicSource(api:*mut ResonanceAudioApi, num_channels: usize) -> u32;
  pub fn CreateStereoSource(api:*mut ResonanceAudioApi, num_channels: usize) -> u32;
  pub fn DestroySource(api:*mut ResonanceAudioApi, id:u32);
  pub fn SetInterleavedBufferF32(api:*mut ResonanceAudioApi, id:u32,
    audio_buffer_ptr:*const f32, num_channels: usize, num_frames: usize);
  pub fn SetSourcePosition(api:*mut ResonanceAudioApi, id:u32, x:f32,y:f32,z:f32);
  pub fn SetSourceRotation(api:*mut ResonanceAudioApi, id:u32, x:f32,y:f32,z:f32,w:f32);
  pub fn SetSourceVolume(api:*mut ResonanceAudioApi, id:u32, vol:f32);
  pub fn SetSoundObjectSpread(api:*mut ResonanceAudioApi, id:u32, spread_deg:f32);
  pub fn SetSoundObjectOcclusionIntensity(api:*mut ResonanceAudioApi, id:u32, v:f32);
  pub fn SetSourceDistanceModel(api:*mut ResonanceAudioApi, id:u32,
    rolloff:i32, min_d:f32, max_d:f32);
  pub fn EnableRoomEffects(api:*mut ResonanceAudioApi, enable: bool);
  pub fn SetReflectionProperties(api:*mut ResonanceAudioApi, props:*const ReflectionProperties);
  pub fn SetReverbProperties(api:*mut ResonanceAudioApi, props:*const ReverbProperties);
}
```

### 5.2 Safe facade used by engine
```rust
pub struct ResonanceCtx { raw: NonNull<ffi::ResonanceAudioApi>, sr:u32, fpb:usize }
impl ResonanceCtx {
  pub fn new(chans:usize, fpb:usize, sr:u32) -> Result<Self, Error> { /* create + NonNull */ }
  pub fn render_into(&mut self, out:&mut [f32], frames:usize) { /* FillInterleavedOutputBufferF32 */ }
  pub fn set_listener(&mut self, pos:[f32;3], rot:[f32;4]);
  pub fn set_master(&mut self, vol:f32, stereo:bool);
  pub fn create_sound_object(&mut self, mode:RenderingMode) -> SourceId; // also stereo/ambisonic
  pub fn destroy_source(&mut self, id:SourceId);
  pub fn set_source_transform(&mut self, id:SourceId, pos:[f32;3], rot:[f32;4]);
  pub fn set_source_params(&mut self, id:SourceId, vol:f32, spread:f32, occ:f32,
                           rolloff:DistanceModel);
  pub fn set_room(&mut self, refl:ReflectionProps, rvb:ReverbProps, enable:bool);
  pub fn queue_buffer(&mut self, id:SourceId, interleaved:&[f32], chans:usize, frames:usize);
}
```

**Why `ResonanceAudioApi`?** It’s **thread‑safe and non‑blocking** for setting parameters and pulling an interleaved output; perfect for our RT audio callback.

---

## 6) Engine Core (Rust)
### 6.1 Public API (safe, game‑thread only)
```rust
pub struct EngineConfig { pub ambisonic_order: u8, pub sample_rate: Option<u32>, pub buffer_size: Option<usize> }
#[derive(Copy, Clone)] pub struct SourceHandle(pub u32);

pub struct Engine { /* hidden */ }
impl Engine {
  pub fn init(cfg: EngineConfig, backend: Box<dyn AudioBackend>) -> Result<Self, Error>;
  pub fn shutdown(self);

  // Listener & Output
  pub fn set_listener_pose(&self, pos:[f32;3], rot:[f32;4]);
  pub fn set_master_volume(&self, vol:f32);           // 0..1
  pub fn set_stereo_speaker_mode(&self, enabled:bool);

  // Rooms
  pub fn enable_room_effects(&self, enabled:bool);
  pub fn set_reflections(&self, props:ReflectionProps);
  pub fn set_reverb(&self, props:ReverbProps);

  // Sources
  pub fn create_sound_object(&self, mode:RenderingMode) -> SourceHandle;
  pub fn create_stereo(&self) -> SourceHandle;
  pub fn create_ambisonic(&self, num_channels:usize) -> SourceHandle;
  pub fn destroy_source(&self, h:SourceHandle);
  pub fn play_interleaved(&self, h:SourceHandle, samples:Arc<[f32]>, chans:usize);
  pub fn stream_register(&self, h:SourceHandle, decoder:StreamId);
  pub fn set_source_pose(&self, h:SourceHandle, pos:[f32;3], rot:[f32;4]);
  pub fn set_source_params(&self, h:SourceHandle, vol:f32, spread:f32, occlusion:f32,
                           rolloff:DistanceModel, min_d:f32, max_d:f32);

  // Scheduling (sample‑accurate)
  pub fn schedule_at(&self, frame:u64, cmd:Command);
}
```
All methods **enqueue** commands to a lock‑free SPSC queue; RT thread applies them when `now_frame >= cmd.frame`.

### 6.2 Command Types
`SetListenerPose`, `SetMasterVolume`, `SetStereoMode`, `CreateSource(kind)`, `DestroySource`, `SetSourcePose`, `SetSourceParams`, `RoomEnable`, `SetReflections`, `SetReverb`, `QueueBuffer{source, ptr/Arc, chans, frames}`.

### 6.3 Real‑time Render Path (platform callback)
```rust
// in AudioBackend::start closure
let render = move |out: &mut [f32], sr: u32, frames: usize| {
    // 1) Advance clock; compute now_frame.
    scheduler.apply_due_commands(now_frame, |cmd| engine_rt.apply(cmd));

    // 2) Feed fresh audio to Resonance per active source (if new chunks available).
    for s in engine_rt.sources.iter_mut() {
        if let Some(chunk) = s.ring.pop_chunk(frames) {
            resonance.queue_buffer(s.id, &chunk, s.chans, frames);
        }
        resonance.set_source_params(s.id, s.vol, s.spread, s.occlusion, s.rolloff);
        resonance.set_source_transform(s.id, s.pos, s.rot);
    }
    resonance.set_listener(engine_rt.listener.pos, engine_rt.listener.rot);

    // 3) Ask Resonance to render directly into the device buffer.
    resonance.render_into(out, frames);
};
```

---

## 7) ECS & Systems
- **Components**
  - `TransformAudio { pos:[f32;3], rot:[f32;4], vel:[f32;3], room:RoomId }`
  - `AudioSource { handle:SourceHandle, route:Route, gain:f32, spread:f32, occlusion:f32,
                   rolloff:DistanceModel, min_d:f32, max_d:f32 }`
  - `AudioListener { pos, rot, master:f32, stereo:bool }`
  - `Room { bounds:Convex/AABB, reflections:ReflectionProps, reverb:ReverbProps }`
  - `Portal/Occluder { geom, material }`

- **Systems**
  - `RoomSystem`: detect listener transitions → `SetReflections/SetReverb/RoomEnable`.
  - `OcclusionSystem`: cheap ray or sector tests → update source `occlusion` and LPF proxy.
  - `SequencerSystem`: tempo map; resolves musical time to **frame** timestamps → schedules `Command`s.
  - `SpatialSyncSystem`: mirrors ECS poses into engine via `SetListenerPose/SetSourcePose`.

---

## 8) Mapping to Resonance Audio SDK (API Cross‑Ref)
| Engine action | Resonance call |
|---|---|
| Create context | `CreateResonanceAudioApi(num_channels=2, frames_per_buffer, sample_rate_hz)` |
| Render block | `FillInterleavedOutputBuffer(num_channels=2, num_frames, out_f32)` |
| Listener pose | `SetHeadPosition(x,y,z)` + `SetHeadRotation(x,y,z,w)` |
| Master volume | `SetMasterVolume(volume)` |
| Stereo speaker | `SetStereoSpeakerMode(enabled)` |
| Create source (object) | `CreateSoundObjectSource(RenderingMode)` |
| Create stereo source | `CreateStereoSource(num_channels)` |
| Create ambisonic source | `CreateAmbisonicSource(num_channels)` |
| Destroy source | `DestroySource(id)` |
| Feed audio | `SetInterleavedBuffer(source_id, audio_ptr, num_channels, num_frames)` |
| Source position/rotation | `SetSourcePosition/SetSourceRotation` |
| Source volume | `SetSourceVolume(volume)` |
| Distance model | `SetSourceDistanceModel(rolloff, min_distance, max_distance)` |
| Distance attenuation (explicit) | `SetSourceDistanceAttenuation(value)` |
| Spread | `SetSoundObjectSpread(spread_deg)` |
| Occlusion | `SetSoundObjectOcclusionIntensity(intensity)` |
| Room enable | `EnableRoomEffects(enable)` |
| Room reflections | `SetReflectionProperties(props)` |
| Room reverb | `SetReverbProperties(props)` |

> Note: We **don’t** manipulate internal graph nodes at runtime. The API is high‑level and thread‑safe, matching our callback model.

---

## 9) Content & Streaming
- **Assets**: WAV/FLAC/OGG → decode to f32; pre‑resample to device SR when feasible; otherwise resample offline and cache.
- **Streaming**: Long files stream via per‑source lock‑free ring buffers sized to ~100–200 ms. The RT thread only pops available frames.
- **Ambisonic**: Multi‑channel assets (FuMa/ACN+SN3D) mapped to RA `CreateAmbisonicSource` with validation.
- **Metadata**: loop points, gain staging, loudness, route (stereo/ui vs object vs ambisonic).

---

## 10) Rooms, Materials, Occlusion
- Listener enters `Room` bounds → enqueue `EnableRoomEffects(true)` and update reflections/reverb with crossfade over N ms (frame aligned).
- Material presets map to occlusion & HF damping → drive `SetSoundObjectOcclusionIntensity` and optional post‑EQ (engine‑side if needed).
- Portals: snap sources to proxy room or apply attenuation preset.

---

## 11) Accessibility & Mix Policy
- **Ducking**: When VO active, reduce Music/SFX via engine bus gains (pre‑Resonance volumes) for clarity.
- **Dynamic Range**: Night/Normal/Loud profiles via per‑source volumes and optional engine‑side limiter before handoff to Resonance.
- **Mono fold‑down**: toggle `SetStereoSpeakerMode(true)` when on loudspeakers.

---

## 12) Diagnostics
- XRun telemetry (callback underruns), adaptive buffer increase on repeated XRuns.
- Output meter and crest factor; scene validation (sample rate match, channel layouts).
- DOT dump of ECS + audio routing; per‑platform latency probes (loopback test harness).

---

## 13) Build & Packaging
- **Windows**: `cargo build -p app-windows` (links `resonance-ffi` static lib).
- **Android**: `cargo ndk -t arm64-v8a -o ./target/android build -p app-android`; AAR with JNI loader that instantiates `OboeBackend` and passes the render closure from Rust.
- CMake (for FFI shim) invoked via `build.rs`; headers for `ResonanceAudioApi` included in repo or submodule.

---

## 14) Minimal Example (pseudo‑Rust)
```rust
let mut backend = CpalBackend::new(None)?; // picks default device (48k)
let mut engine = Engine::init(EngineConfig{ ambisonic_order:1, sample_rate:None, buffer_size:None },
                              Box::new(backend))?;

let s = engine.create_sound_object(RenderingMode::HighQualityBinaural);
engine.set_source_pose(s, [1.0,0.0,0.0], [0.0,0.0,0.0,1.0]);
engine.set_source_params(s, 0.8, 15.0, 0.0, DistanceModel::InverseSquare, 1.0, 20.0);
engine.play_interleaved(s, load_clip("door.wav")?, 1);

engine.set_listener_pose([0.0,0.0,0.0], quat_from_yaw(0.0));
```

---

## 15) Testing Plan
- **Unit**: command ordering, frame scheduler, ring buffer gaps, parameter smoothing.
- **DSP**: null rendering, denormals off, buffer wrap correctness.
- **Performance**: 128 concurrent mono sources @ 48 kHz on mid‑tier Android; no XRuns.
- **Rooms**: enter/exit crossfade, property updates under load.

---

## 16) Risks & Mitigations
- **FFI drift**: pin Resonance headers; generate bindings in CI; keep a stable C shim API.
- **Buffer starvation**: enforce min fill level; back‑pressure streaming.
- **Device variability**: adaptive buffer sizes; voice‑capping; content loudness normalization.

---

## 17) Implementation Checklist
- [ ] `resonance-ffi`: raw externs + safe `ResonanceCtx` wrapper.
- [ ] `audio-backend`: Oboe + CPAL implementations.
- [ ] Lock‑free command queue + frame scheduler.
- [ ] Source registry, ring buffers, clip/stream decoders.
- [ ] Public `Engine` API + ECS systems (Rooms, Occlusion, Sequencer, SpatialSync).
- [ ] Content loader (RON/JSON) + validation.
- [ ] Telemetry + XRun handling; limiter (optional) before render.
- [ ] Android JNI wrapper + Windows launcher app.

---

### Appendix A — Resonance Calls We Use (verbatim from SDK surface)
- **Creation/Render**: `CreateResonanceAudioApi`, `FillInterleavedOutputBuffer` (f32/i16)
- **Listener**: `SetHeadPosition`, `SetHeadRotation`, `SetMasterVolume`, `SetStereoSpeakerMode`
- **Sources**: `CreateAmbisonicSource`, `CreateStereoSource`, `CreateSoundObjectSource`, `DestroySource`
- **Feed Buffers**: `SetInterleavedBuffer` (f32/i16), `SetPlanarBuffer` (f32/i16)
- **Per‑Source Params**: `SetSourceDistanceAttenuation`, `SetSourceDistanceModel`, `SetSourcePosition`, `SetSourceRoomEffectsGain`, `SetSourceRotation`, `SetSourceVolume`, `SetSoundObjectDirectivity`, `SetSoundObjectListenerDirectivity`, `SetSoundObjectNearFieldEffectGain`, `SetSoundObjectOcclusionIntensity`, `SetSoundObjectSpread`
- **Room Effects**: `EnableRoomEffects`, `SetReflectionProperties`, `SetReverbProperties`

> The engine maps 1:1 to these methods via the safe `ResonanceCtx` facade and schedules all mutations at block boundaries on the audio callback thread.

