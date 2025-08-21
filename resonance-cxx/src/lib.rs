pub mod bridge;

pub use bridge::{ResonanceAudioApi, DistanceRolloffModel, ReflectionProperties, RenderingMode, ReverbProperties};

use std::pin::Pin;
use cxx::UniquePtr;

// Safety: The upstream C++ `vraudio::ResonanceAudioApi` is documented in
// `resonance_audio_api.h` as thread-safe for concurrent calls from the audio
// thread and the main/render thread. Declaring the generated opaque type as
// `Send`/`Sync` allows the Rust side to move/share the FFI object across
// threads. This is an unsafe assertion: callers must still ensure that no
// thread will call into the C++ object while its destructor is running
// (for example, join backend worker threads before dropping the Api).
unsafe impl Send for bridge::ResonanceAudioApi {}
unsafe impl Sync for bridge::ResonanceAudioApi {}

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

	/// Fill a planar buffer (channels separated) by converting to an
	/// interleaved temporary and calling the interleaved fill implementation.
	///
	/// Accepts `channels` as a slice of mutable slices, one per channel,
	/// each of length `num_frames`. Returns false if lengths mismatch.
	pub fn fill_planar_f32(&mut self, channels: &mut [&mut [f32]]) -> bool {
		if channels.is_empty() {
			return true; // nothing to do
		}
		let num_channels = channels.len();
		let num_frames = channels[0].len();
		for ch in channels.iter() {
			if ch.len() != num_frames { return false; }
		}

		// Create an interleaved temporary buffer
		let mut interleaved = vec![0f32; num_channels * num_frames];
		// Call the interleaved fill which writes into the buffer
		let ok = self.as_pin_mut().fill_interleaved_output_buffer_f32(num_channels, num_frames, &mut interleaved);
		if !ok { return false; }

		// Deinterleave into planar slices
		for frame in 0..num_frames {
			for ch in 0..num_channels {
				channels[ch][frame] = interleaved[frame * num_channels + ch];
			}
		}
		true
	}

	/// Set a planar source buffer (immutable) by interleaving into a temporary
	/// and calling the interleaved setter. This avoids exposing raw pointers
	/// across the FFI and keeps the public surface safe.
	pub fn set_planar_buffer_f32(&mut self, source_id: i32, channels: &[&[f32]], num_frames: usize) -> bool {
		if channels.is_empty() { return true; }
		let num_channels = channels.len();
		for ch in channels.iter() { if ch.len() != num_frames { return false; } }

		let mut interleaved = vec![0f32; num_channels * num_frames];
		for frame in 0..num_frames {
			for ch in 0..num_channels {
				interleaved[frame * num_channels + ch] = channels[ch][frame];
			}
		}
		self.as_pin_mut().set_interleaved_buffer_f32(source_id, &interleaved, num_channels, num_frames);
		true
	}

	/// Planar helpers for i16 audio.
	pub fn fill_planar_i16(&mut self, channels: &mut [&mut [i16]]) -> bool {
		if channels.is_empty() { return true; }
		let num_channels = channels.len();
		let num_frames = channels[0].len();
		for ch in channels.iter() { if ch.len() != num_frames { return false; } }

		let mut interleaved = vec![0i16; num_channels * num_frames];
		let ok = self.as_pin_mut().fill_interleaved_output_buffer_i16(num_channels, num_frames, &mut interleaved);
		if !ok { return false; }
		for frame in 0..num_frames {
			for ch in 0..num_channels {
				channels[ch][frame] = interleaved[frame * num_channels + ch];
			}
		}
		true
	}

	pub fn set_planar_buffer_i16(&mut self, source_id: i32, channels: &[&[i16]], num_frames: usize) -> bool {
		if channels.is_empty() { return true; }
		let num_channels = channels.len();
		for ch in channels.iter() { if ch.len() != num_frames { return false; } }
		let mut interleaved = vec![0i16; num_channels * num_frames];
		for frame in 0..num_frames {
			for ch in 0..num_channels {
				interleaved[frame * num_channels + ch] = channels[ch][frame];
			}
		}
		self.as_pin_mut().set_interleaved_buffer_i16(source_id, &interleaved, num_channels, num_frames);
		true
	}

	/// Variant that accepts a caller-provided interleaved scratch buffer for f32.
	/// The scratch buffer will be resized as needed. Using this avoids the
	/// allocation per-call in high-frequency paths.
	pub fn set_planar_buffer_f32_with_scratch(&mut self, source_id: i32, channels: &[&[f32]], num_frames: usize, scratch: &mut Vec<f32>) -> bool {
		if channels.is_empty() { return true; }
		let num_channels = channels.len();
		for ch in channels.iter() { if ch.len() != num_frames { return false; } }
		let needed = num_channels * num_frames;
		if scratch.len() < needed { scratch.resize(needed, 0.0); }
		for frame in 0..num_frames {
			for ch in 0..num_channels {
				scratch[frame * num_channels + ch] = channels[ch][frame];
			}
		}
		self.as_pin_mut().set_interleaved_buffer_f32(source_id, &scratch, num_channels, num_frames);
		true
	}

	pub fn set_planar_buffer_i16_with_scratch(&mut self, source_id: i32, channels: &[&[i16]], num_frames: usize, scratch: &mut Vec<i16>) -> bool {
		if channels.is_empty() { return true; }
		let num_channels = channels.len();
		for ch in channels.iter() { if ch.len() != num_frames { return false; } }
		let needed = num_channels * num_frames;
		if scratch.len() < needed { scratch.resize(needed, 0); }
		for frame in 0..num_frames {
			for ch in 0..num_channels {
				scratch[frame * num_channels + ch] = channels[ch][frame];
			}
		}
		self.as_pin_mut().set_interleaved_buffer_i16(source_id, &scratch, num_channels, num_frames);
		true
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

	pub fn set_source_room_effects_gain(&mut self, source_id: i32, room_effects_gain: f32) { self.as_pin_mut().set_source_room_effects_gain(source_id, room_effects_gain); }

	pub fn set_source_rotation(&mut self, source_id: i32, x: f32, y: f32, z: f32, w: f32) { self.as_pin_mut().set_source_rotation(source_id, x, y, z, w); }

	pub fn set_source_volume(&mut self, source_id: i32, volume: f32) { self.as_pin_mut().set_source_volume(source_id, volume); }

	pub fn set_sound_object_directivity(&mut self, source_id: i32, alpha: f32, order: f32) { self.as_pin_mut().set_sound_object_directivity(source_id, alpha, order); }

	pub fn set_sound_object_listener_directivity(&mut self, source_id: i32, alpha: f32, order: f32) { self.as_pin_mut().set_sound_object_listener_directivity(source_id, alpha, order); }

	pub fn set_sound_object_near_field_effect_gain(&mut self, source_id: i32, gain: f32) { self.as_pin_mut().set_sound_object_near_field_effect_gain(source_id, gain); }

	pub fn set_sound_object_occlusion_intensity(&mut self, source_id: i32, intensity: f32) { self.as_pin_mut().set_sound_object_occlusion_intensity(source_id, intensity); }

	pub fn set_sound_object_spread(&mut self, source_id: i32, spread_deg: f32) { self.as_pin_mut().set_sound_object_spread(source_id, spread_deg); }

	pub fn enable_room_effects(&mut self, enable: bool) { self.as_pin_mut().enable_room_effects(enable); }

	pub fn set_reflection_properties(&mut self, props: &ReflectionProperties) { self.as_pin_mut().set_reflection_properties(props); }

	pub fn set_reverb_properties(&mut self, props: &ReverbProperties) { self.as_pin_mut().set_reverb_properties(props); }
}

