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

        // Output filling (interleaved float/int16 using Rust slices)
        #[rust_name = "fill_interleaved_output_buffer_f32"]
        fn FillInterleavedOutputBufferF32(
            self: Pin<&mut ResonanceAudioApi>,
            num_channels: usize,
            num_frames: usize,
            buffer: &mut [f32],
        ) -> bool;

        #[rust_name = "fill_interleaved_output_buffer_i16"]
        fn FillInterleavedOutputBufferI16(
            self: Pin<&mut ResonanceAudioApi>,
            num_channels: usize,
            num_frames: usize,
            buffer: &mut [i16],
        ) -> bool;

        // Planar output (each channel is a separate buffer pointer)
        /// # Safety
        /// - `buffers` must point to an array of `num_channels` valid, writable pointers
        ///   to `f32` buffers, each with at least `num_frames` elements.
        /// - The pointed-to memory must remain valid for the duration of the call.
        /// - Caller guarantees proper alignment and that no aliasing UB occurs.
        #[rust_name = "fill_planar_output_buffer_f32"]
        unsafe fn FillPlanarOutputBufferF32(
            self: Pin<&mut ResonanceAudioApi>,
            num_channels: usize,
            num_frames: usize,
            buffers: *const *mut f32,
        ) -> bool;

        /// # Safety
        /// - `buffers` must point to an array of `num_channels` valid, writable pointers
        ///   to `i16` buffers, each with at least `num_frames` elements.
        /// - The pointed-to memory must remain valid for the duration of the call.
        /// - Caller guarantees proper alignment and that no aliasing UB occurs.
        #[rust_name = "fill_planar_output_buffer_i16"]
        unsafe fn FillPlanarOutputBufferI16(
            self: Pin<&mut ResonanceAudioApi>,
            num_channels: usize,
            num_frames: usize,
            buffers: *const *mut i16,
        ) -> bool;

        // Head / global
        #[rust_name = "set_head_position"]
        fn SetHeadPosition(self: Pin<&mut ResonanceAudioApi>, x: f32, y: f32, z: f32);

        #[rust_name = "set_head_rotation"]
        fn SetHeadRotation(self: Pin<&mut ResonanceAudioApi>, x: f32, y: f32, z: f32, w: f32);

        #[rust_name = "set_master_volume"]
        fn SetMasterVolume(self: Pin<&mut ResonanceAudioApi>, volume: f32);

        #[rust_name = "set_stereo_speaker_mode"]
        fn SetStereoSpeakerMode(self: Pin<&mut ResonanceAudioApi>, enabled: bool);

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

        #[rust_name = "create_stereo_source"]
        fn CreateStereoSource(self: Pin<&mut ResonanceAudioApi>, num_channels: usize) -> i32;

        #[rust_name = "create_sound_object_source"]
        fn CreateSoundObjectSource(
            self: Pin<&mut ResonanceAudioApi>,
            rendering_mode: RenderingMode,
        ) -> i32;

        #[rust_name = "destroy_source"]
        fn DestroySource(self: Pin<&mut ResonanceAudioApi>, id: i32);

        #[rust_name = "set_interleaved_buffer_f32"]
        fn SetInterleavedBufferF32(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            audio: &[f32],
            num_channels: usize,
            num_frames: usize,
        );

        #[rust_name = "set_interleaved_buffer_i16"]
        fn SetInterleavedBufferI16(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            audio: &[i16],
            num_channels: usize,
            num_frames: usize,
        );

        // Planar buffer entrypoints accepting array-of-pointers from the C++ side.
        // We expose pointer-based planar setters so high-performance callers can
        // build per-channel pointers in C++ and call directly.
        /// # Safety
        /// - `audio_ptrs` must point to an array of `num_channels` valid pointers to
        ///   `f32` channel data, each with at least `num_frames` elements.
        /// - The pointed-to memory must remain valid for the duration of the call.
        /// - Caller guarantees proper alignment and that no aliasing UB occurs.
        #[rust_name = "set_planar_buffer_f32_ptrs"]
        unsafe fn SetPlanarBufferF32(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            audio_ptrs: *const *const f32,
            num_channels: usize,
            num_frames: usize,
        );

        /// # Safety
        /// - `audio_ptrs` must point to an array of `num_channels` valid pointers to
        ///   `i16` channel data, each with at least `num_frames` elements.
        /// - The pointed-to memory must remain valid for the duration of the call.
        /// - Caller guarantees proper alignment and that no aliasing UB occurs.
        #[rust_name = "set_planar_buffer_i16_ptrs"]
        unsafe fn SetPlanarBufferI16(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            audio_ptrs: *const *const i16,
            num_channels: usize,
            num_frames: usize,
        );

        #[rust_name = "set_source_distance_attenuation"]
        fn SetSourceDistanceAttenuation(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            distance_attenuation: f32,
        );

        #[rust_name = "set_source_distance_model"]
        fn SetSourceDistanceModel(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            rolloff: DistanceRolloffModel,
            min_distance: f32,
            max_distance: f32,
        );

        #[rust_name = "set_source_position"]
        fn SetSourcePosition(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            x: f32,
            y: f32,
            z: f32,
        );

        #[rust_name = "set_source_room_effects_gain"]
        fn SetSourceRoomEffectsGain(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            room_effects_gain: f32,
        );

        #[rust_name = "set_source_rotation"]
        fn SetSourceRotation(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            x: f32,
            y: f32,
            z: f32,
            w: f32,
        );

        #[rust_name = "set_source_volume"]
        fn SetSourceVolume(self: Pin<&mut ResonanceAudioApi>, source_id: i32, volume: f32);

        #[rust_name = "set_sound_object_directivity"]
        fn SetSoundObjectDirectivity(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            alpha: f32,
            order: f32,
        );

        #[rust_name = "set_sound_object_listener_directivity"]
        fn SetSoundObjectListenerDirectivity(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            alpha: f32,
            order: f32,
        );

        #[rust_name = "set_sound_object_near_field_effect_gain"]
        fn SetSoundObjectNearFieldEffectGain(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            gain: f32,
        );

        #[rust_name = "set_sound_object_occlusion_intensity"]
        fn SetSoundObjectOcclusionIntensity(
            self: Pin<&mut ResonanceAudioApi>,
            source_id: i32,
            intensity: f32,
        );

        #[rust_name = "set_sound_object_spread"]
        fn SetSoundObjectSpread(self: Pin<&mut ResonanceAudioApi>, source_id: i32, spread_deg: f32);

        #[rust_name = "enable_room_effects"]
        fn EnableRoomEffects(self: Pin<&mut ResonanceAudioApi>, enable: bool);
    }
}

// Re-export the ffi symbols at the `bridge` module level for convenience.
pub use ffi::{
    DistanceRolloffModel, ReflectionProperties, RenderingMode, ResonanceAudioApi, ReverbProperties,
};
