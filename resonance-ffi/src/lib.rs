// Generated FFI bindings for ResonanceAudioApi
// Import FFI symbols from generated bindings
#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code, improper_ctypes)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// Publicly re-export generated bindings so other crates can import them
pub mod bindings {
    pub use super::ffi::*;
}

use bindings::{resonance_create_api, resonance_destroy_api, ResonanceAudioApiHandle};

pub struct ResonanceCtx {
    // Opaque C handle to the ResonanceAudioApi instance
    api: ResonanceAudioApiHandle,
}

impl ResonanceCtx {
    /// Creates a new ResonanceCtx instance.
    pub fn new(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> Result<Self, String> {
        unsafe {
            let api = resonance_create_api(num_channels, frames_per_buffer, sample_rate_hz);
            if api.is_null() {
                return Err("Failed to create ResonanceAudioApi instance".to_string());
            }
            Ok(Self { api })
        }
    }

    /// Cleans up the ResonanceCtx instance by calling the C wrapper destroy.
    pub fn destroy(self) {
        unsafe {
            resonance_destroy_api(self.api);
        }
    }
}

impl Drop for ResonanceCtx {
    fn drop(&mut self) {
        unsafe {
            if !self.api.is_null() {
                resonance_destroy_api(self.api);
            }
        }
    }
}
