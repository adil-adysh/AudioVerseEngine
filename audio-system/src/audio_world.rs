use std::collections::HashMap;
use resonance_cxx::{Api, DistanceRolloffModel, RenderingMode, ReflectionProperties, ReverbProperties};

pub type Vec3 = [f32; 3];
pub type Quat = [f32; 4];

/// Component representing a spatial audio source
pub struct AudioSource {
    pub source_id: i32,
    pub _rendering_mode: RenderingMode,
    pub gain: f32,
    pub distance_model: DistanceRolloffModel,
    pub directivity_alpha: f32,
    pub directivity_order: f32,
    pub spread_deg: f32,
}

/// Component representing an entity's position and rotation in 3D space
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
}

/// Represents a zone or room in the world
pub struct AudioZone {
    pub min: Vec3,
    pub max: Vec3,
    pub reverb: ReverbProperties,
    pub reflection: ReflectionProperties,
    pub room_effect_gain: f32,
    pub occlusion: f32,
}

/// ECS-style world managing all audio
pub struct AudioWorld {
    api: Api,
    listener_position: Vec3,
    listener_rotation: Quat,
    transforms: HashMap<u32, Transform>,     // entity -> Transform
    audio_sources: HashMap<u32, AudioSource>,// entity -> AudioSource
    zones: Vec<AudioZone>,                   // world zones/rooms
    active_source_keys: Vec<u32>,
    // Parallel vector to `active_source_keys` containing the native source id
    // for each active entity. This allows the RT path to forward buffers
    // directly to the native Api without HashMap lookups.
    active_source_ids: Vec<i32>,
}

impl AudioWorld {
    /// Create a new AudioWorld
    pub fn new(api: Api) -> Self {
        Self {
            api,
            listener_position: [0.0, 0.0, 0.0],
            listener_rotation: [0.0, 0.0, 0.0, 1.0],
            transforms: HashMap::new(),
            audio_sources: HashMap::new(),
            zones: Vec::new(),
            // Preallocate a small capacity to avoid initial reallocations on the RT path.
            active_source_keys: Vec::with_capacity(64),
            active_source_ids: Vec::with_capacity(64),
        }
    }

    // --- Public API-forwarding helpers for adapters ---
    pub fn api_create_sound_object_source(&mut self, mode: resonance_cxx::RenderingMode) -> i32 {
        self.api.create_sound_object_source(mode)
    }

    pub fn api_destroy_source(&mut self, id: i32) {
        self.api.destroy_source(id);
    }

    pub fn api_set_interleaved_buffer_f32(&mut self, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize) {
        self.api.set_interleaved_buffer_f32(source_id, audio, num_channels, num_frames);
    }

    pub fn api_fill_interleaved_f32(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool {
        self.api.fill_interleaved_f32(num_channels, num_frames, buffer)
    }

    pub fn api_set_source_position(&mut self, source_id: i32, x: f32, y: f32, z: f32) {
        self.api.set_source_position(source_id, x, y, z);
    }

    pub fn api_set_source_rotation(&mut self, source_id: i32, x: f32, y: f32, z: f32, w: f32) {
        self.api.set_source_rotation(source_id, x, y, z, w);
    }

    pub fn api_set_source_distance_model(&mut self, source_id: i32, rolloff: resonance_cxx::DistanceRolloffModel, min_distance: f32, max_distance: f32) {
        self.api.set_source_distance_model(source_id, rolloff, min_distance, max_distance);
    }

    pub fn api_set_source_volume(&mut self, source_id: i32, volume: f32) {
        self.api.set_source_volume(source_id, volume);
    }

    pub fn api_set_sound_object_directivity(&mut self, source_id: i32, alpha: f32, order: f32) {
        self.api.set_sound_object_directivity(source_id, alpha, order);
    }

    pub fn api_set_source_room_effects_gain(&mut self, source_id: i32, gain: f32) {
        self.api.set_source_room_effects_gain(source_id, gain);
    }

    pub fn api_set_source_distance_attenuation(&mut self, source_id: i32, att: f32) {
        self.api.set_source_distance_attenuation(source_id, att);
    }

    pub fn api_set_sound_object_spread(&mut self, source_id: i32, spread_deg: f32) {
        self.api.set_sound_object_spread(source_id, spread_deg);
    }

    pub fn api_set_sound_object_occlusion_intensity(&mut self, source_id: i32, intensity: f32) {
        self.api.set_sound_object_occlusion_intensity(source_id, intensity);
    }

    pub fn api_set_reverb_properties(&mut self, props: &resonance_cxx::ReverbProperties) {
        self.api.set_reverb_properties(props);
    }

    pub fn api_set_reflection_properties(&mut self, props: &resonance_cxx::ReflectionProperties) {
        self.api.set_reflection_properties(props);
    }

    pub fn api_set_head_position(&mut self, x: f32, y: f32, z: f32) {
        self.api.set_head_position(x, y, z);
    }

    pub fn api_set_head_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.api.set_head_rotation(x, y, z, w);
    }

    /// Add an entity's transform
    pub fn add_transform(&mut self, entity: u32, position: Vec3, rotation: Quat) {
        self.transforms.insert(entity, Transform { position, rotation });
    }

    /// Add a spatial audio source to an entity
    pub fn add_audio_source(&mut self, entity: u32, mode: RenderingMode) {
        let source_id = self.api.create_sound_object_source(mode);
        self.audio_sources.insert(entity, AudioSource {
            source_id,
            _rendering_mode: mode,
            gain: 1.0,
            distance_model: DistanceRolloffModel::kLogarithmic,
            directivity_alpha: 0.0,
            directivity_order: 1.0,
            spread_deg: 360.0,
        });
        // Keep the active keys cache in sync. Reserve if needed to avoid reallocating
        // while the audio thread is running.
        if self.active_source_keys.len() == self.active_source_keys.capacity() {
            // grow capacity ahead of time; this path is non-RT (main thread) so allocate here.
            self.active_source_keys.reserve(64);
            self.active_source_ids.reserve(64);
        }
        self.active_source_keys.push(entity);
        self.active_source_ids.push(source_id);
    }

    /// Add a zone/room
    pub fn add_zone(&mut self, zone: AudioZone) {
        self.zones.push(zone);
    }

    /// Update listener position & rotation
    pub fn update_listener(&mut self, position: Vec3, rotation: Quat) {
        self.listener_position = position;
        self.listener_rotation = rotation;
        self.api.set_head_position(position[0], position[1], position[2]);
        self.api.set_head_rotation(rotation[0], rotation[1], rotation[2], rotation[3]);
    }

    /// Update a single entity's transform
    pub fn update_transform(&mut self, entity: u32, position: Vec3, rotation: Quat) {
        if let Some(trans) = self.transforms.get_mut(&entity) {
            trans.position = position;
            trans.rotation = rotation;
        }
    }

    /// Feed audio samples for an entity
    pub fn feed_audio(&mut self, entity: u32, audio: &[f32], num_channels: usize, num_frames: usize) {
        if let Some(source) = self.audio_sources.get(&entity) {
            self.api.set_interleaved_buffer_f32(source.source_id, audio, num_channels, num_frames);
        }
    }

    /// RT-friendly audio feeding helper. Avoids allocations and intermediate
    /// structures; intended for use from the audio/render thread.
    pub fn feed_audio_rt(&mut self, entity: u32, audio: &[f32], num_channels: usize, num_frames: usize) {
        if let Some(source) = self.audio_sources.get(&entity) {
            self.api.set_interleaved_buffer_f32(source.source_id, audio, num_channels, num_frames);
        }
    }

    /// Prepare the internal caches for use on the real-time thread. This should be called
    /// once before the audio/render thread starts reading via `get_active_source_keys`.
    /// It performs any necessary reservations to avoid allocations on the RT path.
    pub fn prepare_for_rt(&mut self) {
        // Ensure some spare capacity so push/pop during non-RT operations don't cause
        // the RT-side iteration to reallocate when the vector grows.
        if self.active_source_keys.capacity() == 0 {
            self.active_source_keys.reserve(64);
        }
    }

    /// RT-friendly accessor: returns a reference to the active source keys vector so the
    /// audio/render thread can iterate without allocations. Iteration must be done carefully
    /// and without mutating the vector from the RT thread.
    pub fn get_active_source_keys(&self) -> &[u32] {
        &self.active_source_keys
    }

    /// RT-friendly batch setter: caller provides a slice of entity ids and a parallel
    /// slice of audio buffers (each `&[f32]` interleaved). This method performs no
    /// allocations and simply forwards the provided buffers into the native `Api`.
    ///
    /// Usage: prepare arrays off-RT, then call this from the audio/render thread.
    /// The arrays must be the same length; excess entities will be ignored.
    pub fn feed_entities_interleaved_rt(&mut self, entities: &[u32], buffers: &[&[f32]], num_channels: usize, num_frames: usize) {
        for (&entity, &audio) in entities.iter().zip(buffers.iter()) {
            if let Some(source) = self.audio_sources.get(&entity) {
                // Forward without allocation
                self.api.set_interleaved_buffer_f32(source.source_id, audio, num_channels, num_frames);
            }
        }
    }

    /// RT-friendly batch forward using the internal active caches. The caller must
    /// supply a slice of interleaved buffers aligned with `get_active_source_keys()`.
    /// This avoids HashMap lookups entirely on the audio thread.
    pub fn feed_active_buffers_rt(&mut self, buffers: &[&[f32]], num_channels: usize, num_frames: usize) {
        for (&sid, &audio) in self.active_source_ids.iter().zip(buffers.iter()) {
            // Direct forward to native API using cached id
            self.api.set_interleaved_buffer_f32(sid, audio, num_channels, num_frames);
        }
    }

    /// ECS-style update: automatically syncs all sources with transforms, applies zone effects, and distance attenuation
    pub fn update(&mut self) {
        for &entity in &self.active_source_keys {
            if let Some(source) = self.audio_sources.get(&entity) {
                if let Some(transform) = self.transforms.get(&entity) {
                    let pos = transform.position;
                    let rot = transform.rotation;

                    // Update position & rotation
                    self.api.set_source_position(source.source_id, pos[0], pos[1], pos[2]);
                    self.api.set_source_rotation(source.source_id, rot[0], rot[1], rot[2], rot[3]);

                    // Apply distance model & volume
                    self.api.set_source_distance_model(source.source_id, source.distance_model, 1.0, 100.0);
                    self.api.set_source_volume(source.source_id, source.gain);

                    // Apply directivity & spread
                    self.api.set_sound_object_directivity(source.source_id, source.directivity_alpha, source.directivity_order);
                    self.api.set_sound_object_spread(source.source_id, source.spread_deg);

                    // Automatically check which zone the source is in
                    for zone in &self.zones {
                        let inside = pos[0] >= zone.min[0] && pos[0] <= zone.max[0]
                            && pos[1] >= zone.min[1] && pos[1] <= zone.max[1]
                            && pos[2] >= zone.min[2] && pos[2] <= zone.max[2];
                        if inside {
                            self.api.set_source_room_effects_gain(source.source_id, zone.room_effect_gain);
                            self.api.set_sound_object_occlusion_intensity(source.source_id, zone.occlusion);
                            self.api.set_reverb_properties(&zone.reverb);
                            self.api.set_reflection_properties(&zone.reflection);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Remove an entity's audio source
    pub fn remove_audio_source(&mut self, entity: u32) {
        if let Some(source) = self.audio_sources.remove(&entity) {
            self.api.destroy_source(source.source_id);
            if let Some(pos) = self.active_source_keys.iter().position(|&e| e == entity) {
                self.active_source_keys.swap_remove(pos);
            }
        }
    }
}
