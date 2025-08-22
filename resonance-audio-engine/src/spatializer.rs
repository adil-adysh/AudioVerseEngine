use resonance_cxx::Api;
use crate::bridge::ffi;

pub struct Spatializer<'a> {
    api: &'a mut Api,
    source_id: i32,
}

impl<'a> Spatializer<'a> {
    /// Create a new spatializer (sound object). Borrow the Renderer Api.
    pub fn new(renderer: &'a mut crate::renderer::Renderer, rendering_mode: ffi::RenderingMode) -> Self {
        let api = renderer.api_mut();
        let src_id = api.create_sound_object_source(rendering_mode);
        Self { api, source_id: src_id }
    }

    /// Feed interleaved audio for this source (read-only).
    pub fn feed_interleaved(&mut self, audio: &[f32], num_channels: usize, num_frames: usize) {
        self.api.set_interleaved_buffer_f32(self.source_id, audio, num_channels, num_frames);
    }

    /// Feed planar audio (borrowed slices per channel).
    pub fn feed_planar(&mut self, channels: &[&[f32]], num_frames: usize) -> bool {
        self.api.set_planar_buffer_f32(self.source_id, channels, num_frames)
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.api.set_source_volume(self.source_id, gain);
    }

    pub fn set_distance_rolloff(&mut self, model: ffi::DistanceRolloffModel) {
        // map to resonance-cxx distance model type
        self.api.set_source_distance_model(self.source_id, model, 1.0, 100.0);
    }

    pub fn destroy(self) {
        let id = self.source_id;
        self.api.destroy_source(id);
    }
}
