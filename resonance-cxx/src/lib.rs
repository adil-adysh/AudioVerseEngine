pub mod bridge;

pub use bridge::{ResonanceAudioApi, DistanceRolloffModel, ReflectionProperties, RenderingMode, ReverbProperties};

use std::pin::Pin;
use cxx::UniquePtr;

/// Safe, ergonomic owner for the underlying C++ `ResonanceAudioApi`.
pub struct Api {
	inner: UniquePtr<ResonanceAudioApi>,
}

impl Api {
	/// Create a new Api instance. Returns None on allocation failure.
	pub fn new(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> Option<Self> {
		let up = bridge::ffi::create_resonance_audio_api(num_channels, frames_per_buffer, sample_rate_hz);
		if up.is_null() { None } else { Some(Api { inner: up }) }
	}

	fn as_pin_mut(&mut self) -> Pin<&mut bridge::ResonanceAudioApi> {
		self.inner.pin_mut()
	}

	pub fn fill_interleaved_f32(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool {
	// Expect buffer.len() == num_channels * num_frames
	self.as_pin_mut().fill_interleaved_output_buffer_f32(num_channels, num_frames, buffer)
	}

	pub fn fill_interleaved_i16(&mut self, num_channels: usize, num_frames: usize, buffer: &mut [i16]) -> bool {
	self.as_pin_mut().fill_interleaved_output_buffer_i16(num_channels, num_frames, buffer)
	}

	pub fn set_head_position(&mut self, x: f32, y: f32, z: f32) { self.as_pin_mut().set_head_position(x, y, z); }
	pub fn set_head_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) { self.as_pin_mut().set_head_rotation(x, y, z, w); }
	pub fn set_master_volume(&mut self, volume: f32) { self.as_pin_mut().set_master_volume(volume); }
	pub fn set_stereo_speaker_mode(&mut self, enabled: bool) { self.as_pin_mut().set_stereo_speaker_mode(enabled); }

	pub fn create_ambisonic_source(&mut self, num_channels: usize) -> i32 { self.as_pin_mut().create_ambisonic_source(num_channels) }
	pub fn create_stereo_source(&mut self, num_channels: usize) -> i32 { self.as_pin_mut().create_stereo_source(num_channels) }
	pub fn create_sound_object_source(&mut self, mode: RenderingMode) -> i32 { self.as_pin_mut().create_sound_object_source(mode) }
	pub fn destroy_source(&mut self, id: i32) { self.as_pin_mut().destroy_source(id); }

	pub fn set_interleaved_buffer_f32(&mut self, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize) {
		self.as_pin_mut().set_interleaved_buffer_f32(source_id, audio, num_channels, num_frames);
	}

	pub fn set_interleaved_buffer_i16(&mut self, source_id: i32, audio: &[i16], num_channels: usize, num_frames: usize) {
		self.as_pin_mut().set_interleaved_buffer_i16(source_id, audio, num_channels, num_frames);
	}

	pub fn set_source_distance_attenuation(&mut self, source_id: i32, distance_attenuation: f32) { self.as_pin_mut().set_source_distance_attenuation(source_id, distance_attenuation); }

	pub fn set_source_distance_model(&mut self, source_id: i32, rolloff: DistanceRolloffModel, min_distance: f32, max_distance: f32) { self.as_pin_mut().set_source_distance_model(source_id, rolloff, min_distance, max_distance); }

	pub fn set_source_position(&mut self, source_id: i32, x: f32, y: f32, z: f32) { self.as_pin_mut().set_source_position(source_id, x, y, z); }

	pub fn enable_room_effects(&mut self, enable: bool) { self.as_pin_mut().enable_room_effects(enable); }

	pub fn set_reflection_properties(&mut self, props: &ReflectionProperties) { self.as_pin_mut().set_reflection_properties(props); }

	pub fn set_reverb_properties(&mut self, props: &ReverbProperties) { self.as_pin_mut().set_reverb_properties(props); }
}

