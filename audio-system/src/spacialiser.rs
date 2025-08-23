use crate::Vec3;
use resonance_cxx::{Api, RenderingMode, DistanceRolloffModel};

/// Local spatialiser that can either perform a simple in-Rust stereo panning
/// or forward to a native `resonance_cxx::Api` source when attached.
#[derive(Default, Debug, Clone)]
pub struct Spatialiser {
    pub listener_position: Vec3,
}

/// Thin handle that owns a native source id and a mutable borrow of the Api.
pub struct NativeSpatializer<'a> {
    api: &'a mut Api,
    source_id: i32,
}

impl<'a> NativeSpatializer<'a> {
    /// Create a new native spatializer (sound object) using the provided Api.
    pub fn new(api: &'a mut Api, rendering_mode: RenderingMode) -> Self {
        let id = api.create_sound_object_source(rendering_mode);
        Self { api, source_id: id }
    }

    pub fn feed_interleaved(&mut self, audio: &[f32], num_channels: usize, num_frames: usize) {
        self.api
            .set_interleaved_buffer_f32(self.source_id, audio, num_channels, num_frames);
    }

    pub fn feed_planar(&mut self, channels: &[&[f32]], num_frames: usize) -> bool {
        self.api.set_planar_buffer_f32(self.source_id, channels, num_frames)
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.api.set_source_volume(self.source_id, gain);
    }

    pub fn set_distance_rolloff(&mut self, model: DistanceRolloffModel) {
        self.api
            .set_source_distance_model(self.source_id, model, 1.0, 100.0);
    }

    pub fn set_pose(&mut self, x: f32, y: f32, z: f32, qx: f32, qy: f32, qz: f32, qw: f32) {
        self.api.set_source_position(self.source_id, x, y, z);
        self.api.set_source_rotation(self.source_id, qx, qy, qz, qw);
    }

    pub fn set_room_effects_gain(&mut self, gain: f32) {
        self.api.set_source_room_effects_gain(self.source_id, gain);
    }

    pub fn set_distance_attenuation(&mut self, attenuation: f32) {
        self.api
            .set_source_distance_attenuation(self.source_id, attenuation);
    }

    pub fn destroy(self) {
        let id = self.source_id;
        self.api.destroy_source(id);
    }
}

impl Spatialiser {
    pub fn new() -> Self { Self { listener_position: [0.0, 0.0, 0.0] } }

    pub fn set_listener_position(&mut self, pos: Vec3) {
        self.listener_position = pos;
    }

    /// Fallback mono->stereo panner if native API is not attached.
    pub fn process_mono_to_stereo(&self, src: &[f32], out: &mut [f32], source_pos: Vec3, base_gain: f32) {
        let dx = source_pos[0] - self.listener_position[0];
        let dz = source_pos[2] - self.listener_position[2];
        let az = dx.atan2(dz);
        let pan = (az / (std::f32::consts::PI / 2.0)).clamp(-1.0, 1.0);
        let dist = ((dx*dx + (source_pos[1]-self.listener_position[1]).powi(2) + dz*dz) as f32).sqrt().max(0.0001);
        let rolloff = 1.0 / (1.0 + dist * 0.1);
        let left_gain = base_gain * rolloff * ((1.0 - pan) * 0.5);
        let right_gain = base_gain * rolloff * ((1.0 + pan) * 0.5);
        let frames = src.len();
        assert_eq!(out.len(), frames * 2);
        for i in 0..frames {
            let s = src[i];
            out[2*i] = s * left_gain;
            out[2*i + 1] = s * right_gain;
        }
    }
}
