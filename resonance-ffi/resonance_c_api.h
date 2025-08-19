#pragma once
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void* ResonanceAudioApiHandle;

typedef enum {
    kStereoPanning = 0,
    kBinauralLowQuality,
    kBinauralMediumQuality,
    kBinauralHighQuality,
    kRoomEffectsOnly,
} RenderingMode;

typedef enum {
    kLogarithmic = 0,
    kLinear,
    kNone,
} DistanceRolloffModel;

typedef struct {
    float room_position[3];
    float room_rotation[4];
    float room_dimensions[3];
    float cutoff_frequency;
    float coefficients[6];
    float gain;
} ReflectionProperties;

typedef struct {
    float rt60_values[9];
    float gain;
} ReverbProperties;

ResonanceAudioApiHandle resonance_create_api(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz);
void resonance_destroy_api(ResonanceAudioApiHandle handle);

bool resonance_fill_interleaved_output_buffer_f32(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, float* buffer_ptr);
bool resonance_fill_interleaved_output_buffer_i16(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, int16_t* buffer_ptr);
bool resonance_fill_planar_output_buffer_f32(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, float* const* buffer_ptr);
bool resonance_fill_planar_output_buffer_i16(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, int16_t* const* buffer_ptr);

void resonance_set_head_position(ResonanceAudioApiHandle handle, float x, float y, float z);
void resonance_set_head_rotation(ResonanceAudioApiHandle handle, float x, float y, float z, float w);
void resonance_set_master_volume(ResonanceAudioApiHandle handle, float volume);
void resonance_set_stereo_speaker_mode(ResonanceAudioApiHandle handle, bool enabled);

int resonance_create_ambisonic_source(ResonanceAudioApiHandle handle, size_t num_channels);
int resonance_create_stereo_source(ResonanceAudioApiHandle handle, size_t num_channels);
int resonance_create_sound_object_source(ResonanceAudioApiHandle handle, RenderingMode rendering_mode);
void resonance_destroy_source(ResonanceAudioApiHandle handle, int source_id);

void resonance_set_interleaved_buffer_f32(ResonanceAudioApiHandle handle, int source_id, const float* audio_buffer_ptr, size_t num_channels, size_t num_frames);
void resonance_set_interleaved_buffer_i16(ResonanceAudioApiHandle handle, int source_id, const int16_t* audio_buffer_ptr, size_t num_channels, size_t num_frames);
void resonance_set_planar_buffer_f32(ResonanceAudioApiHandle handle, int source_id, const float* const* audio_buffer_ptr, size_t num_channels, size_t num_frames);
void resonance_set_planar_buffer_i16(ResonanceAudioApiHandle handle, int source_id, const int16_t* const* audio_buffer_ptr, size_t num_channels, size_t num_frames);

void resonance_set_source_distance_attenuation(ResonanceAudioApiHandle handle, int source_id, float distance_attenuation);
void resonance_set_source_distance_model(ResonanceAudioApiHandle handle, int source_id, DistanceRolloffModel rolloff, float min_distance, float max_distance);
void resonance_set_source_position(ResonanceAudioApiHandle handle, int source_id, float x, float y, float z);
void resonance_set_source_room_effects_gain(ResonanceAudioApiHandle handle, int source_id, float room_effects_gain);
void resonance_set_source_rotation(ResonanceAudioApiHandle handle, int source_id, float x, float y, float z, float w);
void resonance_set_source_volume(ResonanceAudioApiHandle handle, int source_id, float volume);
void resonance_set_sound_object_directivity(ResonanceAudioApiHandle handle, int source_id, float alpha, float order);
void resonance_set_sound_object_listener_directivity(ResonanceAudioApiHandle handle, int source_id, float alpha, float order);
void resonance_set_sound_object_near_field_effect_gain(ResonanceAudioApiHandle handle, int source_id, float gain);
void resonance_set_sound_object_occlusion_intensity(ResonanceAudioApiHandle handle, int source_id, float intensity);
void resonance_set_sound_object_spread(ResonanceAudioApiHandle handle, int source_id, float spread_deg);
void resonance_enable_room_effects(ResonanceAudioApiHandle handle, bool enable);
void resonance_set_reflection_properties(ResonanceAudioApiHandle handle, const ReflectionProperties* reflection_properties);
void resonance_set_reverb_properties(ResonanceAudioApiHandle handle, const ReverbProperties* reverb_properties);

#ifdef __cplusplus
}
#endif
