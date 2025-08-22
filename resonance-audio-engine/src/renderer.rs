use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crossbeam_queue::ArrayQueue;
use glam::{Vec3, Quat};
use resonance_cxx::{Api, RenderingMode};

use ringbuf::HeapCons;
use ringbuf::traits::Consumer;
use asset_manager::sfx_loader::SfxMetadata;

// ringbuffer implementation is provided by the `ringbuf` crate via HeapRb/HeapCons

// ---------- Config ----------
const MAX_SOURCES: usize = 256;        // pool size (tunable)
const CMD_QUEUE_CAP: usize = 1024;     // bounded command queue

// ---------- Types ----------
#[derive(Debug, Clone)]
pub struct SfxBuffer {
    pub samples: Arc<Vec<f32>>, // interleaved f32 PCM
    pub meta: SfxMetadata,
}

pub enum Command {
    PlaySfx {
        slot: usize,
        buffer: SfxBuffer,
        gain: f32,
        pos: Option<Vec3>,
    },
    StopVoice { slot: usize },
    SetVoiceGain { slot: usize, gain: f32 },
    StartStream { slot: usize, ring: HeapCons<f32>, channels: usize },
    StopStream { slot: usize },
    CreateSource { slot: usize, mode: RenderingMode },
    DestroySource { slot: usize },
    SetListenerPose { position: Vec3, rotation: Quat },
}

pub struct Voice {
    active: AtomicBool,
    sfx: Option<Arc<Vec<f32>>>,
    meta: Option<SfxMetadata>,
    playhead: usize,
    gain: f32,
    spatial_src_id: Option<i32>,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: AtomicBool::new(false),
            sfx: None,
            meta: None,
            playhead: 0,
            gain: 1.0,
            spatial_src_id: None,
        }
    }
}

#[derive(Default)]
pub struct StreamSlot {
    ring: Option<HeapCons<f32>>,
    channels: usize,
    spatial_src_id: Option<i32>,
}


// ---------- Renderer ----------
pub struct Renderer {
    api: Api,
    num_channels: usize,
    frames_per_buffer: usize,

    voices: Vec<Voice>,
    streams: Vec<StreamSlot>,

    sources: Vec<Option<i32>>,

    cmd_queue: Arc<ArrayQueue<Command>>,
    // preallocated scratch to avoid allocations in RT path
    stream_scratch: Vec<f32>,
}

impl Renderer {
    pub fn new(sample_rate_hz: i32, num_channels: usize, frames_per_buffer: usize) -> Self {
        let api = Api::new(num_channels, frames_per_buffer, sample_rate_hz)
            .expect("failed to create resonance Api");

        let mut voices = Vec::with_capacity(MAX_SOURCES);
        voices.resize_with(MAX_SOURCES, Default::default);
        let mut streams = Vec::with_capacity(MAX_SOURCES);
        streams.resize_with(MAX_SOURCES, Default::default);

        Self {
            api,
            num_channels,
            frames_per_buffer,
            voices,
            streams,
            sources: vec![None; MAX_SOURCES],
            cmd_queue: Arc::new(ArrayQueue::new(CMD_QUEUE_CAP)),
            // preallocate stream scratch to avoid heap allocs in RT path
            stream_scratch: vec![0.0f32; frames_per_buffer * num_channels],
        }
    }

    pub fn command_sender(&self) -> Arc<ArrayQueue<Command>> { self.cmd_queue.clone() }

    pub fn alloc_slot(&mut self) -> Option<usize> {
        for (i, v) in self.voices.iter().enumerate() {
            if !v.active.load(Ordering::Acquire) && v.sfx.is_none() && self.sources[i].is_none() {
                return Some(i);
            }
        }
        None
    }

    fn drain_commands(&mut self) {
        for _ in 0..256 {
            match self.cmd_queue.pop() {
                Some(cmd) => self.apply_command(cmd),
                None => break,
            }
        }
    }

    /// Borrow the underlying Api for direct use (used by Spatializer constructor).
    pub(crate) fn api_mut(&mut self) -> &mut resonance_cxx::Api {
        &mut self.api
    }

    fn apply_command(&mut self, cmd: Command) {
        match cmd {
            Command::CreateSource { slot, mode } => {
                if slot < self.sources.len() && self.sources[slot].is_none() {
                    let id = self.api.create_sound_object_source(mode);
                    if id >= 0 {
                        self.sources[slot] = Some(id);
                        self.voices[slot].spatial_src_id = Some(id);
                        self.streams[slot].spatial_src_id = Some(id);
                    }
                }
            }
            Command::DestroySource { slot } => {
                if let Some(Some(id)) = self.sources.get(slot).cloned() {
                    self.api.destroy_source(id);
                    self.sources[slot] = None;
                    self.voices[slot].spatial_src_id = None;
                    self.streams[slot].spatial_src_id = None;
                }
            }
            Command::PlaySfx { slot, buffer, gain, pos } => {
                if slot < self.voices.len() {
                    let v = &mut self.voices[slot];
                    v.sfx = Some(buffer.samples.clone());
                    v.meta = Some(buffer.meta.clone());
                    v.playhead = 0;
                    v.gain = gain;
                    v.active.store(true, Ordering::Release);
                    if let Some(position) = pos {
                        if let Some(Some(src)) = self.sources.get(slot) {
                            self.api.set_source_position(*src, position.x, position.y, position.z);
                        }
                    }
                }
            }
            Command::StopVoice { slot } => {
                if slot < self.voices.len() {
                    let v = &mut self.voices[slot];
                    v.active.store(false, Ordering::Release);
                    v.sfx = None;
                    v.meta = None;
                    v.playhead = 0;
                }
            }
            Command::SetVoiceGain { slot, gain } => {
                if slot < self.voices.len() { self.voices[slot].gain = gain; }
            }
            Command::StartStream { slot, ring, channels } => {
                if slot < self.streams.len() {
                    let s = &mut self.streams[slot];
                    s.ring = Some(ring);
                    s.channels = channels;
                }
            }
            Command::StopStream { slot } => {
                if slot < self.streams.len() {
                    let s = &mut self.streams[slot];
                    s.ring = None;
                    s.channels = 0;
                }
            }
            Command::SetListenerPose { position, rotation } => {
                self.api.set_head_position(position.x, position.y, position.z);
                self.api.set_head_rotation(rotation.x, rotation.y, rotation.z, rotation.w);
            }
        }
    }

    pub fn process_output_interleaved(&mut self, buffer: &mut [f32], num_frames: usize) -> bool {
        self.drain_commands();

    for sample in buffer.iter_mut() { *sample = 0.0; }

        for v in &mut self.voices {
            if !v.active.load(Ordering::Acquire) { continue; }
            if let Some(ref sfx_arc) = v.sfx {
                let samples = &**sfx_arc;
                if let Some(ref meta) = v.meta {
                    let channels = meta.channels as usize;
                    let frames_available = (samples.len() / channels).saturating_sub(v.playhead / channels);
                    let frames_to_mix = frames_available.min(num_frames);
                    for frame in 0..frames_to_mix {
                        let src_base = v.playhead + frame * channels;
                        let dst_base = frame * self.num_channels;
                        for ch in 0..channels.min(self.num_channels) {
                            buffer[dst_base + ch] += samples[src_base + ch] * v.gain;
                        }
                    }
                    v.playhead += frames_to_mix * channels;
                    if v.playhead >= samples.len() {
                        v.active.store(false, Ordering::Release);
                        v.sfx = None;
                        v.meta = None;
                        v.playhead = 0;
                    }
                }
            }
        }

        // stream mixing: reuse preallocated scratch to avoid allocation
        let scratch_len = num_frames * self.num_channels;
        let scratch = &mut self.stream_scratch[..scratch_len];
        for s in &mut self.streams {
            if let Some(ref mut cons) = s.ring {
                let popped = cons.pop_slice(scratch);
                if popped > 0 {
                    for i in 0..popped { buffer[i] += scratch[i]; }
                }
            }
        }

        self.api.fill_interleaved_f32(self.num_channels, num_frames, buffer)
    }
}
