// Generated FFI bindings for ResonanceAudioApi
// Import FFI symbols from generated bindings
#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code, improper_ctypes)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use ffi::{vraudio_CreateResonanceAudioApi, vraudio_ResonanceAudioApi};
pub struct ResonanceCtx {
    // Pointer to the ResonanceAudioApi instance
    api: *mut ffi::vraudio_ResonanceAudioApi,
}

impl ResonanceCtx {
    /// Creates a new ResonanceCtx instance.
    pub fn new(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> Result<Self, String> {
        unsafe {
            let api = vraudio_CreateResonanceAudioApi(num_channels, frames_per_buffer, sample_rate_hz);
            if api.is_null() {
                return Err("Failed to create ResonanceAudioApi instance".to_string());
            }
            Ok(Self { api })
        }
    }

    /// Cleans up the ResonanceAudioApi instance.
    pub fn destroy(self) {
        unsafe {
            // Delete the C++ object using Box::from_raw
            let _ = Box::from_raw(self.api);
        }
    }
}

impl Drop for ResonanceCtx {
    fn drop(&mut self) {
        unsafe {
            if !self.api.is_null() {
                let _ = Box::from_raw(self.api);
            }
        }
    }
}
