
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
}

impl ResonanceAudioApi {
    /// Create a new ResonanceAudioApi instance. Returns None if underlying creation fails.
    pub fn new(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> Option<Self> {
        // The actual FFI function is unsafe; call it but guard a null pointer return.
        let ptr = unsafe { resonance_create_api(num_channels, frames_per_buffer, sample_rate_hz) };
        if ptr.is_null() {
            None
        } else {
            Some(Self { inner: ptr })
        }
    }

    /// Returns whether the inner pointer is non-null.
    pub fn is_valid(&self) -> bool {
        !self.inner.is_null()
    }

    /// Destroy the underlying C API instance. After calling this, the wrapper should not be used.
    pub fn destroy(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                // Call C wrapper
                resonance_destroy_api(self.inner as *mut _);
            }
            self.inner = std::ptr::null_mut();
        }
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
}

/// Re-export C types as Rust aliases for convenience
// Re-export the C-compatible structs from the generated bindings
pub use bindings::ReflectionProperties;
pub use bindings::ReverbProperties;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
