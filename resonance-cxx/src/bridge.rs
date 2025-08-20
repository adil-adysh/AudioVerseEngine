#[cxx::bridge(namespace = "ra")]
pub mod ffi {
    #[repr(i32)]
    pub enum RenderingMode {
        kStereoPanning = 0,
        kBinauralLowQuality,
        kBinauralMediumQuality,
        kBinauralHighQuality,
        kRoomEffectsOnly,
    }

    #[repr(i32)]
    pub enum DistanceRolloffModel {
        kLogarithmic = 0,
        kLinear,
        kNone,
    }

    pub struct ReflectionProperties {
        room_position: [f32; 3],
        room_rotation: [f32; 4],
        room_dimensions: [f32; 3],
        cutoff_frequency: f32,
        coefficients: [f32; 6],
        gain: f32,
    }

    pub struct ReverbProperties {
        rt60_values: [f32; 9],
        gain: f32,
    }

    unsafe extern "C++" {
    include!("resonance-audio-rs/src/api.h");

    // Use `pub type` to declare the C++ class as an opaque type.
    pub type ResonanceAudioApi;
    // Use primitive i32 for source identifiers across the bridge.

        #[rust_name = "create_resonance_audio_api"]
        fn CreateResonanceAudioApi(
            num_channels: usize,
            frames_per_buffer: usize,
            sample_rate_hz: i32,
        ) -> UniquePtr<ResonanceAudioApi>;
        
        // Use Pin<&mut T> for methods that modify the C++ object.
        #[rust_name = "set_reflection_properties"]
        fn SetReflectionProperties(
            self: Pin<&mut ResonanceAudioApi>,
            reflection_properties: &ReflectionProperties,
        );
        
        #[rust_name = "set_reverb_properties"]
        fn SetReverbProperties(
            self: Pin<&mut ResonanceAudioApi>,
            reverb_properties: &ReverbProperties,
        );
        
    #[rust_name = "create_ambisonic_source"]
    fn CreateAmbisonicSource(self: Pin<&mut ResonanceAudioApi>, num_channels: usize) -> i32;
    
    #[rust_name = "destroy_source"]
    fn DestroySource(self: Pin<&mut ResonanceAudioApi>, id: i32);

    #[rust_name = "set_source_position"]
    fn SetSourcePosition(self: Pin<&mut ResonanceAudioApi>, source_id: i32, x: f32, y: f32, z: f32);
    }
}

// Re-export the ffi symbols at the `bridge` module level for convenience.
pub use ffi::{
    ResonanceAudioApi,
    RenderingMode,
    DistanceRolloffModel,
    ReflectionProperties,
    ReverbProperties,
};