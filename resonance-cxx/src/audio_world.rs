use crate::{Api, DistanceRolloffModel, ReflectionProperties, RenderingMode, ReverbProperties};
use glam::{Quat, Vec3};
use std::collections::HashMap;

/// Component representing a spatial audio source
pub struct AudioSource {
    pub source_id: i32,
    pub rendering_mode: RenderingMode,
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
    transforms: HashMap<u32, Transform>, // entity -> Transform
    audio_sources: HashMap<u32, AudioSource>, // entity -> AudioSource
    zones: Vec<AudioZone>,               // world zones/rooms
    // Active source keys cached to enable RT-friendly iteration without
    // allocating temporary collections. Callers should maintain this when
    // adding/removing sources via the provided helpers.
    active_source_keys: Vec<u32>,
}

impl AudioWorld {
    /// Create a new AudioWorld
    pub fn new(api: Api) -> Self {
        Self {
            api,
            listener_position: Vec3::ZERO,
            listener_rotation: Quat::IDENTITY,
            transforms: HashMap::new(),
            audio_sources: HashMap::new(),
            zones: Vec::new(),
            active_source_keys: Vec::new(),
        }
    }

    /// Add an entity's transform
    pub fn add_transform(&mut self, entity: u32, position: Vec3, rotation: Quat) {
        self.transforms
            .insert(entity, Transform { position, rotation });
    }

    /// Add a spatial audio source to an entity
    pub fn add_audio_source(&mut self, entity: u32, mode: RenderingMode) {
        let source_id = self.api.create_sound_object_source(mode);
        self.audio_sources.insert(
            entity,
            AudioSource {
                source_id,
                rendering_mode: mode,
                gain: 1.0,
                distance_model: DistanceRolloffModel::kLogarithmic,
                directivity_alpha: 0.0,
                directivity_order: 1.0,
                spread_deg: 360.0,
            },
        );
        // record active key for RT-safe iteration
        self.active_source_keys.push(entity);
    }

    /// Add a zone/room
    pub fn add_zone(&mut self, zone: AudioZone) {
        self.zones.push(zone);
    }

    /// Update listener position & rotation
    pub fn update_listener(&mut self, position: Vec3, rotation: Quat) {
        self.listener_position = position;
        self.listener_rotation = rotation;
        self.api
            .set_head_position(position.x, position.y, position.z);
        self.api
            .set_head_rotation(rotation.x, rotation.y, rotation.z, rotation.w);
    }

    /// Update a single entity's transform
    pub fn update_transform(&mut self, entity: u32, position: Vec3, rotation: Quat) {
        if let Some(trans) = self.transforms.get_mut(&entity) {
            trans.position = position;
            trans.rotation = rotation;
        }
    }

    /// Feed audio samples for an entity
    pub fn feed_audio(
        &mut self,
        entity: u32,
        audio: &[f32],
        num_channels: usize,
        num_frames: usize,
    ) {
        if let Some(source) = self.audio_sources.get(&entity) {
            self.api
                .set_interleaved_buffer_f32(source.source_id, audio, num_channels, num_frames);
        }
    }

    /// RT-friendly audio feeding helper.
    ///
    /// This method is intended for use from the audio/render thread. It must
    /// not allocate or perform locking. The caller must ensure the `entity`
    /// exists in `audio_sources` (race otherwise). `audio` is borrowed by the
    /// caller and must remain valid for the call duration.
    pub fn feed_audio_rt(
        &mut self,
        entity: u32,
        audio: &[f32],
        num_channels: usize,
        num_frames: usize,
    ) {
        // Direct map to Api FFI which is documented as thread-safe. Avoid any
        // intermediate allocations here.
        if let Some(source) = self.audio_sources.get(&entity) {
            self.api
                .set_interleaved_buffer_f32(source.source_id, audio, num_channels, num_frames);
        }
    }

    /// ECS-style update: automatically syncs all sources with transforms, applies zone effects, and distance attenuation
    pub fn update(&mut self) {
        // Non-RT safe path: iterate via keys cache to avoid temporary iterator
        // allocations when callers have already established active keys.
        for &entity in &self.active_source_keys {
            if let Some(source) = self.audio_sources.get(&entity) {
                if let Some(transform) = self.transforms.get(&entity) {
                    let pos = transform.position;
                    let rot = transform.rotation;

                    // Update position & rotation
                    self.api
                        .set_source_position(source.source_id, pos.x, pos.y, pos.z);
                    self.api
                        .set_source_rotation(source.source_id, rot.x, rot.y, rot.z, rot.w);

                    // Apply distance model & volume
                    self.api.set_source_distance_model(
                        source.source_id,
                        source.distance_model,
                        1.0,
                        100.0,
                    );
                    self.api.set_source_volume(source.source_id, source.gain);

                    // Apply directivity & spread
                    self.api.set_sound_object_directivity(
                        source.source_id,
                        source.directivity_alpha,
                        source.directivity_order,
                    );
                    self.api
                        .set_sound_object_spread(source.source_id, source.spread_deg);

                    // Automatically check which zone the source is in
                    for zone in &self.zones {
                        if pos.cmplt(zone.max).all() && pos.cmpge(zone.min).all() {
                            self.api.set_source_room_effects_gain(
                                source.source_id,
                                zone.room_effect_gain,
                            );
                            self.api.set_sound_object_occlusion_intensity(
                                source.source_id,
                                zone.occlusion,
                            );
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
            // Also remove entity from active keys cache. This is O(n) but
            // removal is expected to be rare compared to per-frame updates.
            if let Some(pos) = self.active_source_keys.iter().position(|&e| e == entity) {
                self.active_source_keys.swap_remove(pos);
            }
        }
    }
}
