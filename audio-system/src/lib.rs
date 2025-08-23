//! Minimal audio-system crate using `oddio` as the audio backend.
//!
//! This crate implements a tiny AudioSystem API inspired by `docs/audio-game-engine.md`.

use anyhow::Result;
use arc_swap::ArcSwapOption;
use crossbeam::queue::ArrayQueue;
use parking_lot::Mutex;
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
    dropped_count: Mutex<u64>,
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
            dropped_count: Mutex::new(0),
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
        *self.dropped_count.lock()
    }
}

impl IMixerProcessor for MixerQueue {
    fn push(&self, cmd: MixerCommand) {
        if self.queue.push(cmd).is_err() {
            // drop and count
            let mut c = self.dropped_count.lock();
            *c += 1;
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
}

impl SineSource {
    fn new(handle: u32, freq: f32, volume: f32, category: String, bus: String, priority: u8, order: u64) -> Self {
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
        })
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
            let sine = Arc::new(SineSource::new(handle, freq, 0.2_f32, source.category.clone(), source.category.clone(), source.priority, order));
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
            for src in snapshot.iter() {
                src.render(out, timing.sample_rate);
            }
        }

        self.mixer.incr_stream_time(frames);
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
}
