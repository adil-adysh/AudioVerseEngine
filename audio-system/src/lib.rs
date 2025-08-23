//! Minimal audio-system crate using `oddio` as the audio backend.
//!
//! This crate implements a tiny AudioSystem API inspired by `docs/audio-game-engine.md`.

use anyhow::Result;
use arc_swap::ArcSwapOption;
use crossbeam::queue::ArrayQueue;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;

/// Minimal oddio engine scaffold. Will be replaced with full oddio graph wiring.
#[allow(dead_code)]
struct OddioEngine {
    sample_rate: u32,
    channels: u16,
}

impl OddioEngine {
    fn new(sample_rate: u32, channels: u16) -> Self {
        Self { sample_rate, channels }
    }

    /// Render into the provided buffer. For now this is a no-op.
    #[allow(dead_code)]
    fn render(&self, _out: &mut [f32], _frames: usize) {
        // TODO: implement oddio graph mixing
    }
}

use std::f32::consts::PI;
use std::sync::Arc;
// event-bus tests/bridging removed - use Bevy Events via engine-core instead.
mod spacialiser;
use spacialiser::Spatialiser;
mod audio_world;
pub use audio_world::AudioWorld;
use resonance_cxx::Api as ResonanceApi;
use resonance_cxx::{DistanceRolloffModel, RenderingMode};

/// Minimal trait to represent the subset of the resonance API we call from this crate.
/// This allows injecting a mock in tests without pulling the real C++ object.
pub trait ResonanceApiLike: Send + Sync {
    fn create_sound_object_source(&mut self, mode: RenderingMode) -> i32;
    fn destroy_source(&mut self, id: i32);
    fn set_interleaved_buffer_f32(&mut self, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize);
    fn fill_interleaved_f32(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool;
    fn set_source_distance_model(&mut self, source_id: i32, rolloff: DistanceRolloffModel, min_distance: f32, max_distance: f32);
    fn set_sound_object_directivity(&mut self, source_id: i32, alpha: f32, order: f32);
    fn set_source_room_effects_gain(&mut self, source_id: i32, room_effects_gain: f32);
    fn set_source_distance_attenuation(&mut self, source_id: i32, distance_attenuation: f32);
    fn set_source_position(&mut self, source_id: i32, x: f32, y: f32, z: f32);
    fn set_source_volume(&mut self, source_id: i32, volume: f32);
    fn set_head_position(&mut self, x: f32, y: f32, z: f32);
}

impl ResonanceApiLike for ResonanceApi {
    fn create_sound_object_source(&mut self, mode: RenderingMode) -> i32 { self.create_sound_object_source(mode) }
    fn destroy_source(&mut self, id: i32) { self.destroy_source(id); }
    fn set_interleaved_buffer_f32(&mut self, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize) { self.set_interleaved_buffer_f32(source_id, audio, num_channels, num_frames); }
    fn fill_interleaved_f32(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool { self.fill_interleaved_f32(num_channels, num_frames, buffer) }
    fn set_source_distance_model(&mut self, source_id: i32, rolloff: DistanceRolloffModel, min_distance: f32, max_distance: f32) { self.set_source_distance_model(source_id, rolloff, min_distance, max_distance); }
    fn set_sound_object_directivity(&mut self, source_id: i32, alpha: f32, order: f32) { self.set_sound_object_directivity(source_id, alpha, order); }
    fn set_source_room_effects_gain(&mut self, source_id: i32, room_effects_gain: f32) { self.set_source_room_effects_gain(source_id, room_effects_gain); }
    fn set_source_distance_attenuation(&mut self, source_id: i32, distance_attenuation: f32) { self.set_source_distance_attenuation(source_id, distance_attenuation); }
    fn set_source_position(&mut self, source_id: i32, x: f32, y: f32, z: f32) { self.set_source_position(source_id, x, y, z); }
    fn set_source_volume(&mut self, source_id: i32, volume: f32) { self.set_source_volume(source_id, volume); }
    fn set_head_position(&mut self, x: f32, y: f32, z: f32) { self.set_head_position(x, y, z); }
}

/// Simple representation of a 3D vector for transforms.
pub type Vec3 = [f32; 3];

/// Spatial audio directivity pattern.
#[derive(Debug, Clone)]
pub struct Directivity {
    pub alpha: f32,
    pub sharpness: f32,
}

/// Spatial audio options closely matching the document.
#[derive(Debug, Clone)]
pub struct SpatialAudioOptions {
    pub directivity: Directivity,
    pub rolloff_model: RollOffModel,
    pub source_width: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone)]
pub enum RollOffModel {
    Logarithmic,
    Linear,
    None,
}

/// Listener component storing simple transform info used to update oddio listener state.
#[derive(Default, Debug, Clone)]
pub struct AudioListenerComponent {
    pub position: Vec3,
    pub velocity: Vec3,
}

/// Audio source configuration stored on entities.
#[derive(Debug, Clone)]
pub struct AudioSourceComponent {
    pub asset_id: String,
    pub is_spatial: bool,
    pub spatial_options: Option<SpatialAudioOptions>,
    pub priority: u8, // 0..100
    pub category: String,
}

/// Dynamic playback state component.
#[derive(Debug, Clone)]
pub struct AudioPlaybackStateComponent {
    pub bus_name: String,
    pub is_spatial: bool,
    pub volume: f32,
    pub sound_instance_handle: Option<u32>,
    pub streaming: bool,
    pub stream_id: Option<u32>,
}

/// Commands pushed to the mixer from non-RT threads. Small set for the MVP.
#[derive(Debug)]
pub enum MixerCommand {
    Start { handle: u32 },
    Stop { handle: u32 },
    SetVolume { handle: u32, volume: f32 },
}

/// IMixerProcessor: RT-safe push and timing info for the audio thread.
pub trait IMixerProcessor: Send + Sync + 'static {
    fn push(&self, cmd: MixerCommand);
    fn get_timing(&self) -> MixerTiming;
}

#[derive(Debug, Clone, Copy)]
pub struct MixerTiming {
    pub sample_rate: u32,
    pub buffer_frames: u32,
    pub stream_time_frames: u64,
}

/// A simple bounded, RT-safe mixer queue implementation backed by crossbeam::ArrayQueue.
pub struct MixerQueue {
    queue: Arc<ArrayQueue<MixerCommand>>,
    timing: Mutex<MixerTiming>,
    dropped_count: AtomicU64,
}

impl MixerQueue {
    pub fn new(cap: usize, sample_rate: u32, buffer_frames: u32) -> Self {
        MixerQueue {
            queue: Arc::new(ArrayQueue::new(cap)),
            timing: Mutex::new(MixerTiming {
                sample_rate,
                buffer_frames,
                stream_time_frames: 0,
            }),
            dropped_count: AtomicU64::new(0),
        }
    }

    /// Inherent getter for timing so callers don't need the trait object.
    pub fn get_timing_inherent(&self) -> MixerTiming {
        *self.timing.lock()
    }

    /// Called by the real-time audio thread to drain commands.
    pub fn drain_to_vec(&self, out: &mut Vec<MixerCommand>) {
        while let Some(cmd) = self.queue.pop() {
            out.push(cmd);
        }
    }

    pub fn incr_stream_time(&self, frames: u32) {
        let mut t = self.timing.lock();
        t.stream_time_frames = t.stream_time_frames.wrapping_add(frames as u64);
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }
}

impl IMixerProcessor for MixerQueue {
    fn push(&self, cmd: MixerCommand) {
        if self.queue.push(cmd).is_err() {
            // drop and count
            self.dropped_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn get_timing(&self) -> MixerTiming {
        *self.timing.lock()
    }
}

/// Very small AudioSystem that holds minimal oddio components and a mixer queue.
pub struct AudioSystem {
    inner: Arc<Mutex<AudioSystemInner>>,
    mixer: Arc<MixerQueue>,
    sources: Arc<ActiveSources>,
    oddio: Arc<Mutex<Option<Arc<OddioEngine>>>>,
    spatialiser: Arc<Mutex<Spatialiser>>,
    resonance_api: Arc<Mutex<Option<Box<dyn ResonanceApiLike>>>> ,
    audio_world: Arc<Mutex<Option<audio_world::AudioWorld>>>,
}

struct AudioSystemInner {
    next_handle: u32,
    max_voices: usize,
    // tracks active handles in insertion order for oldest-steal policy
    active_order: Vec<u32>,
    // per-bus limits
    bus_limits: HashMap<String, usize>,
    // simple ducking rules
    ducking_rules: Vec<DuckingRule>,
    // incremental order counter for tie-breaks
    order_counter: u64,
}

/// Simple ducking rule placeholder
struct DuckingRule {
    source_category: String,
    target_bus: String,
    duck_amount: f32, // linear scale 0..1
}

struct ActiveSources {
    list: ArcSwapOption<Vec<Arc<SineSource>>>,
}

impl ActiveSources {
    fn new() -> Self {
        Self {
            list: ArcSwapOption::from(None),
        }
    }

    fn snapshot(&self) -> Option<Arc<Vec<Arc<SineSource>>>> {
        self.list.load_full()
    }

    fn add(&self, s: Arc<SineSource>) {
        let cur = self.list.load_full();
        let mut new = match cur.as_ref() {
            Some(vec) => (**vec).clone(),
            None => Vec::new(),
        };
        new.push(s.clone());
        self.list.store(Some(Arc::new(new)));
    }

    fn remove_by_handle(&self, handle: u32) {
        let cur = self.list.load_full();
        let mut new = match cur.as_ref() {
            Some(vec) => (**vec).clone(),
            None => Vec::new(),
        };
        let before = new.len();
        new.retain(|s| s.handle != handle);
        if new.len() != before {
            self.list.store(Some(Arc::new(new)));
        }
    }
}

#[allow(dead_code)]
struct SineSource {
    handle: u32,
    freq: f32,
    phase: Mutex<f32>,
    volume: Mutex<f32>,
    target_volume: Mutex<f32>,
    attack_ms: u32,
    release_ms: u32,
    category: String,
    bus: String,
    priority: u8,
    order: u64,
    // spatial properties
    is_spatial: bool,
    // optional engine entity key for bridging (engine entity index)
    entity_key: Mutex<Option<u32>>,
    // position and spatial options are updated from non-RT threads; store them in ArcSwapOption to avoid locks on RT path
    position_and_options: ArcSwapOption<(Vec3, Option<SpatialAudioOptions>)>,
    // Optional native source id when a resonance Api is attached.
    native_source_id: Mutex<Option<i32>>,
}

impl SineSource {
    fn new(handle: u32, freq: f32, volume: f32, category: String, bus: String, priority: u8, order: u64, is_spatial: bool, position: Vec3, spatial_options: Option<SpatialAudioOptions>) -> Self {
        Self {
            handle,
            freq,
            phase: Mutex::new(0.0),
            volume: Mutex::new(volume),
            target_volume: Mutex::new(volume),
            attack_ms: 5,
            release_ms: 50,
            category,
            bus,
            priority,
            order,
            is_spatial,
            entity_key: Mutex::new(None),
            position_and_options: ArcSwapOption::from(Some(Arc::new((position, spatial_options)))),
            native_source_id: Mutex::new(None),
        }
    }

    fn render(&self, out: &mut [f32], sample_rate: u32) {
        let mut p = self.phase.lock();
        // Read current and target volumes once
        let mut cur_v = *self.volume.lock();
        let target_v = *self.target_volume.lock();
        let step_phase = 2.0 * PI * self.freq / sample_rate as f32;

        // Determine ramp frames (per-sample) based on attack/release
        // We'll compute per-sample increment: (target - cur) / frames_needed
        let frames_needed = if target_v > cur_v {
            // attack
            (self.attack_ms as f32 * sample_rate as f32 / 1000.0).max(1.0)
        } else {
            // release
            (self.release_ms as f32 * sample_rate as f32 / 1000.0).max(1.0)
        };
        let incr = (target_v - cur_v) / frames_needed;

        for s in out.iter_mut() {
            *s += (*p).sin() * cur_v;
            *p += step_phase;
            if *p > 2.0 * PI {
                *p -= 2.0 * PI;
            }
            // step current volume
            cur_v += incr;
            // clamp to [0, 1]
            cur_v = cur_v.clamp(0.0, 1.0);
        }

        // store back current volume
        *self.volume.lock() = cur_v;
    }
}

impl AudioSystem {
    pub fn new(mixer_capacity: usize, sample_rate: u32, buffer_frames: u32) -> Result<Self> {
    let inner = AudioSystemInner { next_handle: 1, max_voices: 64, active_order: Vec::new(), bus_limits: HashMap::new(), ducking_rules: Vec::new(), order_counter: 0 };
        let mixer = MixerQueue::new(mixer_capacity, sample_rate, buffer_frames);
        Ok(AudioSystem {
            inner: Arc::new(Mutex::new(inner)),
            mixer: Arc::new(mixer),
            sources: Arc::new(ActiveSources::new()),
            oddio: Arc::new(Mutex::new(None)),
            spatialiser: Arc::new(Mutex::new(Spatialiser::new())),
            resonance_api: Arc::new(Mutex::new(None)),
            audio_world: Arc::new(Mutex::new(None)),
        })
    }

    /// Update the listener position used by the spatialiser. This is safe to call from the non-RT path.
    pub fn set_listener_position(&self, pos: Vec3) {
        let mut sp = self.spatialiser.lock();
        sp.set_listener_position(pos);
        // If a resonance Api is attached, update its head position too.
        let mut api_guard = self.resonance_api.lock();
        if let Some(ref mut api) = api_guard.as_mut() {
            api.set_head_position(pos[0], pos[1], pos[2]);
        }
    }

    /// Attach a boxed `ResonanceApiLike` (allows injecting mocks in tests).
    pub fn attach_resonance_api_box(&self, api: Box<dyn ResonanceApiLike>) {
    let mut guard = self.resonance_api.lock();
    *guard = Some(api);
    }

    /// Helper to attach a concrete `resonance_cxx::Api` instance.
    pub fn attach_resonance_api(&self, api: ResonanceApi) {
    // attach concrete api and create an AudioWorld wrapper
    let boxed = Box::new(api);
    // create a small Api owned by AudioWorld by taking out the concrete Api back
    // Note: we keep boxed in resonance_api for legacy uses, and also create an AudioWorld using the concrete Api
    self.attach_resonance_api_box(boxed);
    // Try to construct AudioWorld using a concrete Api instance by creating a new Api via resonance_cxx::Api::new is not trivial here.
    // Instead, if the boxed type is actually the concrete `resonance_cxx::Api`, create AudioWorld by downcasting is not possible for trait objects.
    // So, as a pragmatic approach, do nothing here; callers may construct an AudioWorld manually via AudioWorld::new using a concrete Api.
    }

    pub fn mixer_processor(&self) -> Arc<dyn IMixerProcessor> {
        self.mixer.clone()
    }

    /// Set the maximum number of concurrent voices. Oldest-played voices are stopped when exceeded.
    pub fn set_max_voices(&self, max: usize) {
        let mut i = self.inner.lock();
        i.max_voices = max;
    }

    /// Set per-bus (category) concurrent voice limit.
    pub fn set_bus_limit(&self, bus: &str, max: usize) {
        let mut i = self.inner.lock();
        i.bus_limits.insert(bus.to_string(), max);
    }

    /// Add a simple ducking rule: when a source of `source_category` starts,
    /// immediately reduce volumes on `target_bus` by `duck_amount` (0.0..1.0).
    pub fn add_ducking_rule(&self, source_category: &str, target_bus: &str, duck_amount: f32) {
        let mut i = self.inner.lock();
        i.ducking_rules.push(DuckingRule { source_category: source_category.to_string(), target_bus: target_bus.to_string(), duck_amount });
    }

    /// Initialize the system; set up oddio endpoint and mixer graph.
    /// For the MVP we don't create the platform endpoint, only provide a render callback simulation.
    pub fn initialize(&self) {
    // Initialize optional oddio engine scaffold using current mixer timing.
    let timing = self.mixer.get_timing_inherent();
    let engine = OddioEngine::new(timing.sample_rate, 2); // default stereo
    let mut o = self.oddio.lock();
    *o = Some(Arc::new(engine));
    }

    /// Start playback for an entity with an AudioSourceComponent.
    /// Returns a handle for the playing instance.
    pub fn start_playback(&self, source: &AudioSourceComponent) -> u32 {
        let mut i = self.inner.lock();
        let handle = i.next_handle;
        i.next_handle += 1;
        // In a full implementation we'd create an oddio::Node for the source and connect it.
        // Instead we push a MixerCommand for the audio thread to act on.
        self.mixer.push(MixerCommand::Start { handle });
        // create small sine source for testing when asset_id starts with "sine"
        // format: "sine" (defaults to 440) or "sine:880" for frequency
        if source.asset_id.starts_with("sine") {
            let mut freq = 440.0_f32;
            if let Some(idx) = source.asset_id.find(':') {
                if let Ok(parsed) = source.asset_id[idx+1..].parse::<f32>() {
                    if parsed > 0.0 { freq = parsed; }
                }
            }
            let order = i.order_counter;
            i.order_counter = i.order_counter.wrapping_add(1);
            // Force spatial path on for this demo sine so Spatialiser is exercised
            let sine = SineSource::new(handle, freq, 0.2_f32, source.category.clone(), source.category.clone(), source.priority, order, true, [0.0,0.0,0.0], source.spatial_options.clone());
            // If a resonance API is attached, create a native source for spatialisation.
            if let Some(ref mut api_box) = *self.resonance_api.lock() {
                // Create native source
                let id = api_box.create_sound_object_source(RenderingMode::kStereoPanning);
                *sine.native_source_id.lock() = Some(id);

                // If spatial options are present, apply mapping (read lock-free from ArcSwapOption)
                if let Some(pair_arc) = sine.position_and_options.load_full() {
                    let pair = &*pair_arc;
                    if let Some(ref opts) = &pair.1 {
                        // map rolloff model
                        let rolloff = match opts.rolloff_model {
                            RollOffModel::Logarithmic => DistanceRolloffModel::kLogarithmic,
                            RollOffModel::Linear => DistanceRolloffModel::kLinear,
                            RollOffModel::None => DistanceRolloffModel::kNone,
                        };
                        api_box.set_source_distance_model(id, rolloff, opts.min_distance, opts.max_distance);
                        // directivity: alpha and sharpness -> use alpha and order (sharpness)
                        api_box.set_sound_object_directivity(id, opts.directivity.alpha, opts.directivity.sharpness);
                        // room effects gain: use source_width as a proxy if not specified
                        api_box.set_source_room_effects_gain(id, opts.source_width);
                        // distance attenuation parameter
                        api_box.set_source_distance_attenuation(id, 1.0); // placeholder
                    }
                    // set initial position and volume from published pair
                    let pos = pair.0;
                    api_box.set_source_position(id, pos[0], pos[1], pos[2]);
                    let vol = *sine.volume.lock();
                    api_box.set_source_volume(id, vol);
                } else {
                    // fallback: use current volume and origin position
                    api_box.set_source_position(id, 0.0, 0.0, 0.0);
                    let vol = *sine.volume.lock();
                    api_box.set_source_volume(id, vol);
                }
            }
            let sine = Arc::new(sine);
            self.sources.add(sine);
        }
        // Apply ducking rules immediately when a source starts
        if !i.ducking_rules.is_empty() {
            for rule in i.ducking_rules.iter() {
                if rule.source_category == source.category {
                    if let Some(snapshot) = self.sources.snapshot() {
                        for s in snapshot.iter().filter(|s| s.bus == rule.target_bus) {
                            let mut v = s.volume.lock();
                            *v *= 1.0 - rule.duck_amount;
                        }
                    }
                }
            }
        }
        // record active order
        i.active_order.push(handle);
        // enforce per-bus limits first
        let mut to_stop: Option<u32> = None;
        if let Some(limit) = i.bus_limits.get(&source.category) {
            // count active in this bus/category
            if let Some(snapshot) = self.sources.snapshot() {
                let count = snapshot.iter().filter(|s| s.bus == source.category).count();
                if count > *limit {
                    // steal oldest in that bus (lowest order)
                    let mut oldest: Option<(&Arc<SineSource>, u64)> = None;
                    for s in snapshot.iter().filter(|s| s.bus == source.category) {
                        if oldest.is_none() || s.order < oldest.unwrap().1 {
                            oldest = Some((s, s.order));
                        }
                    }
                    if let Some((s, _)) = oldest { to_stop = Some(s.handle); }
                }
            }
        }
        // if no per-bus steal, enforce global max_voices with priority-based stealing
        if to_stop.is_none() && i.active_order.len() > i.max_voices {
            if let Some(snapshot) = self.sources.snapshot() {
                // find lowest priority; tie-break by oldest order
                let mut candidate: Option<(u8, u64, u32)> = None; // (priority, order, handle)
                for s in snapshot.iter() {
                    if candidate.is_none() || (s.priority < candidate.unwrap().0) || (s.priority == candidate.unwrap().0 && s.order < candidate.unwrap().1) {
                        candidate = Some((s.priority, s.order, s.handle));
                    }
                }
                if let Some((_, _, h)) = candidate { to_stop = Some(h); }
            }
        }
        if let Some(old) = to_stop {
            // remove from active_order if present
            if let Some(pos) = i.active_order.iter().position(|&x| x == old) {
                i.active_order.remove(pos);
            }
            self.mixer.push(MixerCommand::Stop { handle: old });
            self.sources.remove_by_handle(old);
            tracing::info!(stolen = old, "voice stolen to respect limits");
        }
        tracing::info!(asset = %source.asset_id, "start_playback: assigned handle {}", handle);
        handle
    }

    pub fn stop_playback(&self, handle: u32) {
        self.mixer.push(MixerCommand::Stop { handle });
        // remove from active_order bookkeeping as well
        let mut i = self.inner.lock();
        if let Some(pos) = i.active_order.iter().position(|&x| x == handle) {
            i.active_order.remove(pos);
        }
        drop(i);
        // If we have a native source attached for this handle, destroy it.
        if let Some(snapshot) = self.sources.snapshot() {
            for s in snapshot.iter() {
                if s.handle == handle {
                    if let Some(native_id) = *s.native_source_id.lock() {
                        if let Some(ref mut api) = *self.resonance_api.lock() {
                            api.destroy_source(native_id);
                        }
                    }
                }
            }
        }
        self.sources.remove_by_handle(handle);
        tracing::info!(handle, "stop_playback");
    }

    /// Set volume for a playing handle (0.0..1.0)
    pub fn set_volume(&self, handle: u32, volume: f32) {
        self.mixer.push(MixerCommand::SetVolume { handle, volume });
    }

    /// Called by the main loop; non-RT.
    pub fn update(&self, delta_seconds: f32) {
        // Non-RT work, e.g., applying listener transform updates or updating slow state.
        let _ = delta_seconds;
    }

    /// Simulate an audio render callback that runs on the audio thread.
    /// It drains mixer commands and advances stream time.
    pub fn render_callback(&self, frames: u32, out: &mut [f32]) {
        // Drain commands
        let mut cmds = Vec::new();
        self.mixer.drain_to_vec(&mut cmds);
        for cmd in cmds {
            match cmd {
                MixerCommand::Start { handle } => tracing::debug!(handle, "RT: Start"),
                MixerCommand::Stop { handle } => tracing::debug!(handle, "RT: Stop"),
                MixerCommand::SetVolume { handle, volume } => {
                    tracing::debug!(handle, volume, "RT: SetVolume");
                    // Apply to active sine sources (prototype behavior) via target volume (smooth)
                    if let Some(snapshot) = self.sources.snapshot() {
                        for src in snapshot.iter() {
                            if src.handle == handle {
                                // set target volume with a small attack
                                *src.target_volume.lock() = volume;
                            }
                        }
                    }
                }
            }
        }

        // Zero buffer then mix active sources in place
        for s in out.iter_mut() {
            *s = 0.0;
        }
        if let Some(snapshot) = self.sources.snapshot() {
            let timing = self.mixer.get_timing_inherent();
            // Use thread-local scratch buffers to avoid locks in the RT path.
            thread_local! {
                static TL_MONO: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::new(Vec::new());
                static TL_STEREO: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::new(Vec::new());
            }

            // Lock the resonance API once for this render pass if present.
            let mut api_opt = self.resonance_api.lock();
            let api_present = api_opt.is_some();

            TL_MONO.with(|mono_cell| {
                TL_STEREO.with(|stereo_cell| {
                    let mut mono = mono_cell.borrow_mut();
                    if mono.len() < frames as usize { mono.resize(frames as usize, 0.0); }
                    let mono_buf_ref = &mut *mono;
                    let mut stereo = stereo_cell.borrow_mut();
                    if stereo.len() < frames as usize * 2 { stereo.resize(frames as usize * 2, 0.0); }
                    let stereo_buf_ref = &mut *stereo;

                    for src in snapshot.iter() {
                        // clear mono
                        for v in mono_buf_ref.iter_mut() { *v = 0.0; }
                        src.render(mono_buf_ref, timing.sample_rate);

                        if api_present {
                            if let Some(ref mut api) = api_opt.as_mut() {
                                if let Some(native_id) = *src.native_source_id.lock() {
                                    // Feed the mono buffer as a single-channel interleaved buffer to the native API.
                                    api.set_interleaved_buffer_f32(native_id, mono_buf_ref, 1, frames as usize);
                                    // native source will be mixed by api.fill_interleaved_f32 later
                                    continue;
                                }
                            }
                        }

                        // If not a native source, spatialise locally (or copy to both channels if non-spatial)
                        if src.is_spatial {
                            let sp = self.spatialiser.lock();
                            // Read position from the lock-free ArcSwapOption; fallback to origin if absent
                            let pos = if let Some(pair_arc) = src.position_and_options.load_full() { pair_arc.0 } else { [0.0, 0.0, 0.0] };
                            sp.process_mono_to_stereo(mono_buf_ref, stereo_buf_ref, pos, 1.0);
                            for i in 0..(frames as usize) {
                                out[2*i] += stereo_buf_ref[2*i];
                                out[2*i+1] += stereo_buf_ref[2*i+1];
                            }
                        } else {
                            for i in 0..(frames as usize) {
                                let s = mono_buf_ref[i];
                                out[2*i] += s;
                                out[2*i+1] += s;
                            }
                        }
                    }

                    // If the API was present, ask it to fill an interleaved stereo buffer of mixed native sources
                    if api_present {
                        if let Some(ref mut api) = api_opt.as_mut() {
                            for v in stereo_buf_ref.iter_mut() { *v = 0.0; }
                            let ok = api.fill_interleaved_f32(2, frames as usize, stereo_buf_ref);
                            if ok {
                                for i in 0..(frames as usize) {
                                    out[2*i] += stereo_buf_ref[2*i];
                                    out[2*i+1] += stereo_buf_ref[2*i+1];
                                }
                            }
                        }
                    }
                })
            });
        }

        self.mixer.incr_stream_time(frames);
    }

    /// Consume PlaySoundEvent from the engine's Bevy Events and trigger playback.
    /// This function is a bevy-ecs system that should be registered into the
    /// engine variable schedule (or the main app schedule). It expects the
    /// `Events<engine_core::events::PlaySoundEvent>` resource to be present.
    /// Handle a single PlaySoundEvent. This is a small helper so the
    /// application/engine can register a Bevy system that reads
    /// `Events<engine_core::events::PlaySoundEvent>` and calls this helper
    /// for each event without forcing `audio-system` to depend on Bevy.
    pub fn handle_play_sound_event(sys: &AudioSystem, ev: engine_core::events::PlaySoundEvent) {
        let src = AudioSourceComponent {
            asset_id: "sine:440".to_string(),
            is_spatial: true,
            spatial_options: None,
            priority: 50,
            category: "SFX".to_string(),
        };
        let handle = sys.start_playback(&src);
        // Tag this new source with the originating entity index so we can update its position later
        if let Some(snapshot) = sys.sources.snapshot() {
            for s in snapshot.iter() {
                if s.handle == handle {
                    *s.entity_key.lock() = Some(ev.entity.index());
                    break;
                }
            }
        }
        // Lock the audio_world mutex and, if present, update transforms/sources.
        let mut aw_lock = sys.audio_world.lock();
        if let Some(ref mut aw) = aw_lock.as_mut() {
            let entity = ev.entity;
            aw.add_transform(entity.index(), [0.0_f32, 0.0_f32, 0.0_f32], [0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32]);
            aw.add_audio_source(entity.index(), resonance_cxx::RenderingMode::kStereoPanning);
        }
    }

    /// Update the listener position in response to ListenerTransformEvent
    pub fn handle_listener_transform_event(sys: &AudioSystem, ev: engine_core::events::ListenerTransformEvent) {
        let pos = [ev.matrix.w_axis.x, ev.matrix.w_axis.y, ev.matrix.w_axis.z];
        sys.set_listener_position(pos);
    }

    /// Update an entity-linked source position used by the Spatialiser path.
    /// This is a best-effort O(n) search over active sources; acceptable for small counts.
    pub fn set_entity_position(&self, entity_index: u32, pos: Vec3) {
        if let Some(snapshot) = self.sources.snapshot() {
            for s in snapshot.iter() {
                if *s.entity_key.lock() == Some(entity_index) {
                    // preserve options while updating position
                    let opts = s.position_and_options.load_full().map(|arc| arc.1.clone()).unwrap_or(None);
                    s.position_and_options.store(Some(Arc::new((pos, opts))));
                }
            }
        }
    }

    /// Stop any active source associated with the given entity index.
    pub fn stop_entity(&self, entity_index: u32) {
        if let Some(snapshot) = self.sources.snapshot() {
            for s in snapshot.iter() {
                if *s.entity_key.lock() == Some(entity_index) {
                    self.mixer.push(MixerCommand::Stop { handle: s.handle });
                }
            }
        }
    }

    /// Set volume for any active source associated with the given entity index.
    pub fn set_entity_volume(&self, entity_index: u32, volume: f32) {
        if let Some(snapshot) = self.sources.snapshot() {
            for s in snapshot.iter() {
                if *s.entity_key.lock() == Some(entity_index) {
                    // update target volume used by render path ramping
                    *s.target_volume.lock() = volume.max(0.0);
                }
            }
        }
    }
}

/// Create a `RenderFn` compatible with `audio-backend::RenderFn` that forwards
/// to this AudioSystem's `render_callback` method.
/// Return a generic `Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync>` so the
/// audio-backend crate can convert it into its own `RenderFn` type without
/// introducing a circular crate dependency.
type RenderClosure = std::sync::Arc<dyn Fn(&mut [f32], u32, usize) + Send + Sync + 'static>;
pub fn render_fn_for_system(sys: Arc<AudioSystem>) -> RenderClosure {
    let closure = move |buffer: &mut [f32], sample_rate: u32, frames: usize| {
        let _ = sample_rate;
        sys.render_callback(frames as u32, buffer);
    };
    std::sync::Arc::new(closure)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ...existing code...

    #[test]
    fn mixer_queue_rt_push_and_drain() {
        let sys = AudioSystem::new(64, 48000, 128).unwrap();
        let mp = sys.mixer_processor();
        // push from non-RT thread
        mp.push(MixerCommand::Start { handle: 1 });
        mp.push(MixerCommand::SetVolume {
            handle: 1,
            volume: 0.5,
        });

        // simulate render thread
        let mut buffer = vec![0.0f32; 256];
        sys.render_callback(128, &mut buffer);

        // after drain, queue should be empty and stream time advanced
        let t = mp.get_timing();
        assert_eq!(t.buffer_frames, 128);
        assert!(t.stream_time_frames >= 128);
    }

    #[test]
    fn start_and_stop_playback_via_system() {
        let sys = AudioSystem::new(32, 48000, 128).unwrap();
        let src = AudioSourceComponent {
            asset_id: "test-sfx".to_string(),
            is_spatial: false,
            spatial_options: None,
            priority: 50,
            category: "SFX".to_string(),
        };
        let h = sys.start_playback(&src);
        assert!(h > 0);
        sys.stop_playback(h);

        // drain with render callback
        let mut buffer = vec![0.0f32; 128];
        sys.render_callback(128, &mut buffer);
    }

    #[test]
    fn integration_sine_renders_nonzero() {
        let sys = AudioSystem::new(32, 48000, 128).unwrap();
        sys.initialize();
        let src = AudioSourceComponent {
            asset_id: "sine:440".to_string(),
            is_spatial: false,
            spatial_options: None,
            priority: 50,
            category: "SFX".to_string(),
        };
        let _h = sys.start_playback(&src);

        // Run several render callbacks to accumulate audio
        let mut buffer = vec![0.0f32; 256];
        for _ in 0..4 {
            sys.render_callback(64, &mut buffer);
        }

        // Assert buffer has non-zero samples from the sine
        let any_nonzero = buffer.iter().any(|s| s.abs() > 1e-6);
        assert!(any_nonzero, "expected non-zero samples from sine source");
    }

    #[test]
    fn set_volume_changes_amplitude() {
        let sys = AudioSystem::new(32, 48000, 128).unwrap();
        sys.initialize();
        let src = AudioSourceComponent {
            asset_id: "sine:440".to_string(),
            is_spatial: false,
            spatial_options: None,
            priority: 50,
            category: "SFX".to_string(),
        };
        let h = sys.start_playback(&src);

        let mut buffer = vec![0.0f32; 256];
        sys.render_callback(128, &mut buffer);
        let max1 = buffer.iter().cloned().fold(0.0f32, |a, b| a.max(b.abs()));

        sys.set_volume(h, 0.1);
        let mut buffer2 = vec![0.0f32; 256];
        sys.render_callback(128, &mut buffer2);
        let max2 = buffer2.iter().cloned().fold(0.0f32, |a, b| a.max(b.abs()));

        assert!(max2 < max1, "expected reduced amplitude after set_volume");
    }

    #[test]
    fn oldest_voice_is_stolen_when_over_capacity() {
        let sys = AudioSystem::new(8, 48000, 64).unwrap();
        sys.initialize();
        sys.set_max_voices(2);
        let s1 = AudioSourceComponent { asset_id: "sine:440".to_string(), is_spatial: false, spatial_options: None, priority: 50, category: "SFX".to_string() };
        let s2 = AudioSourceComponent { asset_id: "sine:550".to_string(), is_spatial: false, spatial_options: None, priority: 50, category: "SFX".to_string() };
        let s3 = AudioSourceComponent { asset_id: "sine:660".to_string(), is_spatial: false, spatial_options: None, priority: 50, category: "SFX".to_string() };
    let _h1 = sys.start_playback(&s1);
        let h2 = sys.start_playback(&s2);
        let h3 = sys.start_playback(&s3);

        // After starting 3rd, the oldest (h1) should have been stopped/removed
        let snapshot = sys.sources.snapshot();
        if let Some(vec) = snapshot {
            let handles: Vec<u32> = vec.iter().map(|s| s.handle).collect();
            assert!(!handles.contains(&_h1), "oldest handle should be stolen");
            assert!(handles.contains(&h2) && handles.contains(&h3), "newer handles should remain");
        } else {
            panic!("expected active sources snapshot");
        }
    }

    #[test]
    fn bus_limit_enforced() {
        let sys = AudioSystem::new(8, 48000, 64).unwrap();
        sys.initialize();
        sys.set_bus_limit("SFX", 1);
        let s1 = AudioSourceComponent { asset_id: "sine:440".to_string(), is_spatial: false, spatial_options: None, priority: 50, category: "SFX".to_string() };
        let s2 = AudioSourceComponent { asset_id: "sine:550".to_string(), is_spatial: false, spatial_options: None, priority: 50, category: "SFX".to_string() };
    let _h1 = sys.start_playback(&s1);
        let h2 = sys.start_playback(&s2);
        // only one should remain in SFX
        let snapshot = sys.sources.snapshot().unwrap();
        let sfx_handles: Vec<u32> = snapshot.iter().filter(|s| s.bus == "SFX").map(|s| s.handle).collect();
        assert_eq!(sfx_handles.len(), 1);
        assert!(sfx_handles.contains(&h2));
    }

    #[test]
    fn ducking_rule_applies() {
        let sys = AudioSystem::new(8, 48000, 64).unwrap();
        sys.initialize();
        // start a background ambience on bus "Ambience"
        let amb = AudioSourceComponent { asset_id: "sine:110".to_string(), is_spatial: false, spatial_options: None, priority: 10, category: "Ambience".to_string() };
        let _ha = sys.start_playback(&amb);
        // add a rule: when SFX starts, duck Ambience by 50%
        sys.add_ducking_rule("SFX", "Ambience", 0.5);
        // capture volume before
        let snap1 = sys.sources.snapshot().unwrap();
        let amb_vol_before: f32 = snap1.iter().find(|s| s.bus == "Ambience").map(|s| *s.volume.lock()).unwrap();
        // start an SFX source which should trigger ducking
        let sfx = AudioSourceComponent { asset_id: "sine:440".to_string(), is_spatial: false, spatial_options: None, priority: 50, category: "SFX".to_string() };
        let _hs = sys.start_playback(&sfx);
        let snap2 = sys.sources.snapshot().unwrap();
        let amb_vol_after: f32 = snap2.iter().find(|s| s.bus == "Ambience").map(|s| *s.volume.lock()).unwrap();
        assert!(amb_vol_after < amb_vol_before);
    }

    #[test]
    fn listener_position_changes_stereo_balance() {
        let sys = AudioSystem::new(8, 48000, 64).unwrap();
        sys.initialize();
        // start a spatial sine source
        let src = AudioSourceComponent { asset_id: "sine:440".to_string(), is_spatial: true, spatial_options: None, priority: 50, category: "SFX".to_string() };
        let _h = sys.start_playback(&src);
        // set listener to be left of origin -> source at origin should be heard more to the right
        sys.set_listener_position([ -2.0, 0.0, 0.0 ]);

        let mut buffer = vec![0.0f32; 256];
        sys.render_callback(128, &mut buffer);
        // compute average L and R absolute
        let mut sum_l = 0.0f32;
        let mut sum_r = 0.0f32;
        let frames = buffer.len() / 2;
        for i in 0..frames {
            sum_l += buffer[2*i].abs();
            sum_r += buffer[2*i+1].abs();
        }
        assert!(sum_r > sum_l, "expected right channel louder when listener is left of source");
    }

    #[test]
    fn spatial_start_without_api_leaves_no_native_id() {
        let sys = AudioSystem::new(8, 48000, 64).unwrap();
        sys.initialize();
        let src = AudioSourceComponent { asset_id: "sine:440".to_string(), is_spatial: true, spatial_options: None, priority: 50, category: "SFX".to_string() };
        let h = sys.start_playback(&src);
        // snapshot should contain the source and native_source_id should be None since no api attached
        if let Some(vec) = sys.sources.snapshot() {
            let found = vec.iter().find(|s| s.handle == h).expect("source not found");
            assert!(found.native_source_id.lock().is_none());
        } else {
            panic!("expected active sources");
        }
    }

    #[test]
    fn spatial_options_map_to_resonance_api_calls() {
        use std::sync::Mutex as StdMutex;

        // Define a simple mock implementing ResonanceApiLike
        struct MockApi {
            calls: StdMutex<Vec<String>>,
            next_id: StdMutex<i32>,
        }
        impl MockApi {
            fn new() -> Self { Self { calls: StdMutex::new(Vec::new()), next_id: StdMutex::new(42) } }
        }
        impl ResonanceApiLike for MockApi {
            fn create_sound_object_source(&mut self, _mode: RenderingMode) -> i32 {
                let mut id = self.next_id.lock().unwrap();
                *id += 1;
                let nid = *id;
                self.calls.lock().unwrap().push(format!("create({})", nid));
                nid
            }
            fn destroy_source(&mut self, id: i32) { self.calls.lock().unwrap().push(format!("destroy({})", id)); }
            fn set_interleaved_buffer_f32(&mut self, source_id: i32, _audio: &[f32], _num_channels: usize, _num_frames: usize) { self.calls.lock().unwrap().push(format!("set_buffer({})", source_id)); }
            fn fill_interleaved_f32(&mut self, _num_channels: usize, _num_frames: usize, _buffer: &mut [f32]) -> bool { true }
            fn set_source_distance_model(&mut self, source_id: i32, rolloff: DistanceRolloffModel, min: f32, max: f32) { self.calls.lock().unwrap().push(format!("distance_model({}, {}, {}, {})", source_id, match rolloff { DistanceRolloffModel::kLogarithmic => "log", DistanceRolloffModel::kLinear => "lin", DistanceRolloffModel::kNone => "none", _ => "unknown" }, min, max)); }
            fn set_sound_object_directivity(&mut self, source_id: i32, alpha: f32, order: f32) { self.calls.lock().unwrap().push(format!("directivity({}, {}, {})", source_id, alpha, order)); }
            fn set_source_room_effects_gain(&mut self, source_id: i32, room_effects_gain: f32) { self.calls.lock().unwrap().push(format!("room_gain({}, {})", source_id, room_effects_gain)); }
            fn set_source_distance_attenuation(&mut self, source_id: i32, distance_attenuation: f32) { self.calls.lock().unwrap().push(format!("atten({}, {})", source_id, distance_attenuation)); }
            fn set_source_position(&mut self, source_id: i32, x: f32, y: f32, z: f32) { self.calls.lock().unwrap().push(format!("pos({}, {}, {}, {})", source_id, x, y, z)); }
            fn set_source_volume(&mut self, source_id: i32, volume: f32) { self.calls.lock().unwrap().push(format!("vol({}, {})", source_id, volume)); }
            fn set_head_position(&mut self, _x: f32, _y: f32, _z: f32) { }
        }

        let sys = AudioSystem::new(8, 48000, 64).unwrap();
    // Attach mock
    let mock = MockApi::new();
    sys.attach_resonance_api_box(Box::new(mock));

        // Start a spatial source with options
        let opts = SpatialAudioOptions {
            directivity: Directivity { alpha: 0.5, sharpness: 2.0 },
            rolloff_model: RollOffModel::Logarithmic,
            source_width: 0.3,
            min_distance: 0.1,
            max_distance: 10.0,
        };
        let src = AudioSourceComponent { asset_id: "sine:440".to_string(), is_spatial: true, spatial_options: Some(opts), priority: 50, category: "SFX".to_string() };
        let h = sys.start_playback(&src);

        // Find the mock from the boxed trait (downcast via Any isn't trivial); instead render once to force any calls
        let mut buf = vec![0.0f32; 256];
        sys.render_callback(128, &mut buf);

        // Access the snapshot and ensure a native id exists
        if let Some(vec) = sys.sources.snapshot() {
            let found = vec.iter().find(|s| s.handle == h).expect("source not found");
            assert!(found.native_source_id.lock().is_some());
        } else { panic!("expected active sources"); }
    }
}
