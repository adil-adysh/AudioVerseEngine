
// Re-export bindings from the resonance-ffi crate
#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code, unused_imports)]
use resonance_ffi::bindings as bindings;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

use bindings::*;

/// Safe wrapper for ResonanceAudioApi (uses the C wrapper handle)
pub struct ResonanceAudioApi {
    inner: ResonanceAudioApiHandle,
    // Tracks whether the native handle has been destroyed to make destruction
    // idempotent and avoid double-free across Drop and explicit destroy calls.
    destroyed: std::cell::Cell<bool>,
}

impl ResonanceAudioApi {
    /// Create a new ResonanceAudioApi instance. Returns None if underlying creation fails.
    pub fn new(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> Option<Self> {
        // The actual FFI function is unsafe; call it but guard a null pointer return.
        let ptr = unsafe { resonance_create_api(num_channels, frames_per_buffer, sample_rate_hz) };
        if ptr.is_null() {
            None
        } else {
            Some(Self { inner: ptr, destroyed: std::cell::Cell::new(false) })
        }
    }

    /// Returns whether the inner pointer is non-null.
    pub fn is_valid(&self) -> bool {
        !self.inner.is_null()
    }

    /// Destroy the underlying C API instance. This method is idempotent; calling
    /// it multiple times is safe. Prefer calling this when you want deterministic
    /// teardown; otherwise Drop will destroy the native handle when the wrapper
    /// is dropped.
    pub fn destroy(&mut self) {
        if self.inner.is_null() || self.destroyed.get() {
            return;
        }
        unsafe { resonance_destroy_api(self.inner as *mut _) }
        self.inner = std::ptr::null_mut();
        self.destroyed.set(true);
    }

    pub fn set_head_position(&self, x: f32, y: f32, z: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_head_position(self.inner as *mut _, x, y, z) }
        }
    }

    pub fn set_head_rotation(&self, x: f32, y: f32, z: f32, w: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_head_rotation(self.inner as *mut _, x, y, z, w) }
        }
    }

    pub fn set_master_volume(&self, volume: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_master_volume(self.inner as *mut _, volume) }
        }
    }

    pub fn set_stereo_speaker_mode(&self, enabled: bool) {
        if !self.inner.is_null() {
            unsafe { resonance_set_stereo_speaker_mode(self.inner as *mut _, enabled) }
        }
    }

    pub fn enable_room_effects(&self, enable: bool) {
        if !self.inner.is_null() {
            unsafe { resonance_enable_room_effects(self.inner as *mut _, enable) }
        }
    }
    // Source management and properties
    pub fn create_ambisonic_source(&self, num_channels: usize) -> i32 {
        if self.inner.is_null() {
            return -1;
        }
        unsafe { resonance_create_ambisonic_source(self.inner as *mut _, num_channels) }
    }

    pub fn create_stereo_source(&self, num_channels: usize) -> i32 {
        if self.inner.is_null() {
            return -1;
        }
        unsafe { resonance_create_stereo_source(self.inner as *mut _, num_channels) }
    }

    pub fn create_sound_object_source(&self, rendering_mode: i32) -> i32 {
        if self.inner.is_null() {
            return -1;
        }
        unsafe { resonance_create_sound_object_source(self.inner as *mut _, rendering_mode) }
    }

    pub fn destroy_source(&self, source_id: i32) {
        if !self.inner.is_null() {
            unsafe { resonance_destroy_source(self.inner as *mut _, source_id) }
        }
    }

    pub fn set_source_position(&self, source_id: i32, x: f32, y: f32, z: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_source_position(self.inner as *mut _, source_id, x, y, z) }
        }
    }

    pub fn set_source_rotation(&self, source_id: i32, x: f32, y: f32, z: f32, w: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_source_rotation(self.inner as *mut _, source_id, x, y, z, w) }
        }
    }

    pub fn set_source_volume(&self, source_id: i32, volume: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_source_volume(self.inner as *mut _, source_id, volume) }
        }
    }

    pub fn set_source_room_effects_gain(&self, source_id: i32, gain: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_source_room_effects_gain(self.inner as *mut _, source_id, gain) }
        }
    }

    pub fn set_source_distance_attenuation(&self, source_id: i32, distance_attenuation: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_source_distance_attenuation(self.inner as *mut _, source_id, distance_attenuation) }
        }
    }

    pub fn set_source_distance_model(&self, source_id: i32, rolloff: i32, min_distance: f32, max_distance: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_source_distance_model(self.inner as *mut _, source_id, rolloff, min_distance, max_distance) }
        }
    }

    pub fn set_sound_object_directivity(&self, source_id: i32, alpha: f32, order: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_sound_object_directivity(self.inner as *mut _, source_id, alpha, order) }
        }
    }

    pub fn set_sound_object_listener_directivity(&self, source_id: i32, alpha: f32, order: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_sound_object_listener_directivity(self.inner as *mut _, source_id, alpha, order) }
        }
    }

    pub fn set_sound_object_near_field_effect_gain(&self, source_id: i32, gain: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_sound_object_near_field_effect_gain(self.inner as *mut _, source_id, gain) }
        }
    }

    pub fn set_sound_object_occlusion_intensity(&self, source_id: i32, intensity: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_sound_object_occlusion_intensity(self.inner as *mut _, source_id, intensity) }
        }
    }

    pub fn set_sound_object_spread(&self, source_id: i32, spread_deg: f32) {
        if !self.inner.is_null() {
            unsafe { resonance_set_sound_object_spread(self.inner as *mut _, source_id, spread_deg) }
        }
    }

    pub fn set_reflection_properties(&self, props: &ReflectionProperties) {
        if !self.inner.is_null() {
            unsafe { resonance_set_reflection_properties(self.inner as *mut _, props) }
        }
    }

    pub fn set_reverb_properties(&self, props: &ReverbProperties) {
        if !self.inner.is_null() {
            unsafe { resonance_set_reverb_properties(self.inner as *mut _, props) }
        }
    }

    // Buffer-related wrappers
    pub fn set_interleaved_buffer_f32(&self, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize) {
        if self.inner.is_null() { return; }
        // safety: pass pointer to first element (or null if empty) and lengths
        let ptr = if audio.is_empty() { std::ptr::null() } else { audio.as_ptr() };
        unsafe { resonance_set_interleaved_buffer_f32(self.inner as *mut _, source_id, ptr, num_channels, num_frames) }
    }

    pub fn set_interleaved_buffer_i16(&self, source_id: i32, audio: &[i16], num_channels: usize, num_frames: usize) {
        if self.inner.is_null() { return; }
        let ptr = if audio.is_empty() { std::ptr::null() } else { audio.as_ptr() };
        unsafe { resonance_set_interleaved_buffer_i16(self.inner as *mut _, source_id, ptr, num_channels, num_frames) }
    }

    pub fn fill_interleaved_output_buffer_f32(&self, num_channels: usize, num_frames: usize, out: &mut [f32]) -> bool {
        if self.inner.is_null() { return false; }
        let ptr = if out.is_empty() { std::ptr::null_mut() } else { out.as_mut_ptr() };
        unsafe { resonance_fill_interleaved_output_buffer_f32(self.inner as *mut _, num_channels, num_frames, ptr) }
    }

    pub fn fill_interleaved_output_buffer_i16(&self, num_channels: usize, num_frames: usize, out: &mut [i16]) -> bool {
        if self.inner.is_null() { return false; }
        let ptr = if out.is_empty() { std::ptr::null_mut() } else { out.as_mut_ptr() };
        unsafe { resonance_fill_interleaved_output_buffer_i16(self.inner as *mut _, num_channels, num_frames, ptr) }
    }
}

impl Drop for ResonanceAudioApi {
    fn drop(&mut self) {
        // Ensure we only destroy once. `destroy()` already guards null checks,
        // but call the FFI directly here to avoid borrow issues when dropping.
        if !self.inner.is_null() && !self.destroyed.get() {
            unsafe { resonance_destroy_api(self.inner as *mut _) }
            self.inner = std::ptr::null_mut();
            self.destroyed.set(true);
        }
    }
}

/// Re-export C types as Rust aliases for convenience
// Re-export the C-compatible structs from the generated bindings
pub use bindings::ReflectionProperties;
pub use bindings::ReverbProperties;

#[cfg(test)]
mod tests {
    use super::*;

    use std::mem;

    // Helper to construct a ResonanceAudioApi with a null inner handle.
    fn api_with_null() -> ResonanceAudioApi {
    ResonanceAudioApi { inner: std::ptr::null_mut(), destroyed: std::cell::Cell::new(false) }
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn resonance_api_is_valid_checks_pointer() {
        // null handle -> invalid
        let api = api_with_null();
        assert!(!api.is_valid());
    }

    #[test]
    fn resonance_api_noop_methods_on_null_handle() {
        // Ensure methods that would call into FFI are no-ops when the inner
        // handle is null. These should not panic or attempt to deref.
        let api = api_with_null();

        // Calling setters on a null handle should be safe (no-op).
        api.set_head_position(1.0, 2.0, 3.0);
        api.set_head_rotation(0.0, 0.0, 0.0, 1.0);
        api.set_master_volume(0.5);
        api.set_stereo_speaker_mode(true);
        api.enable_room_effects(true);

        // No observable state change; just ensure the calls do not panic.
        assert!(!api.is_valid());
    }

    #[test]
    fn reflection_and_reverb_properties_exist_and_sized() {
        // We don't call the FFI setters here; just ensure the structs are
        // present from the bindings and can be zero-initialized and inspected.
        let refl_size = mem::size_of::<ReflectionProperties>();
        let reverb_size = mem::size_of::<ReverbProperties>();

        // Sanity: sizes should be non-zero
        assert!(refl_size > 0, "ReflectionProperties size must be > 0");
        assert!(reverb_size > 0, "ReverbProperties size must be > 0");

        // Ability to zero-init and access a couple of fields (unsafe required
        // because these types come from bindgen and may not implement Default).
        unsafe {
            let mut refl: ReflectionProperties = mem::zeroed();
            refl.room_position[0] = 1.0;
            refl.room_rotation[3] = 1.0;

            let mut reverb: ReverbProperties = mem::zeroed();
            reverb.rt60_values[0] = 0.1;

            // Basic assertions on fields we set
            assert_eq!(refl.room_position[0], 1.0f32);
            assert_eq!(refl.room_rotation[3], 1.0f32);
            assert_eq!(reverb.rt60_values[0], 0.1f32);
        }
    }
}
