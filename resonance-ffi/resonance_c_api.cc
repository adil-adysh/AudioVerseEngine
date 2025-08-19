#include "resonance_c_api.h"
#include "../resonance-audio/resonance_audio/api/resonance_audio_api.h"

using vraudio::ResonanceAudioApi;
using vraudio::RenderingMode;
using vraudio::DistanceRolloffModel;
using vraudio::ReflectionProperties;
using vraudio::ReverbProperties;

extern "C" {

ResonanceAudioApiHandle resonance_create_api(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz) {
    return CreateResonanceAudioApi(num_channels, frames_per_buffer, sample_rate_hz);
}
void resonance_destroy_api(ResonanceAudioApiHandle handle) {
    delete static_cast<ResonanceAudioApi*>(handle);
}

bool resonance_fill_interleaved_output_buffer_f32(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, float* buffer_ptr) {
    return static_cast<ResonanceAudioApi*>(handle)->FillInterleavedOutputBuffer(num_channels, num_frames, buffer_ptr);
}
bool resonance_fill_interleaved_output_buffer_i16(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, int16_t* buffer_ptr) {
    return static_cast<ResonanceAudioApi*>(handle)->FillInterleavedOutputBuffer(num_channels, num_frames, buffer_ptr);
}
bool resonance_fill_planar_output_buffer_f32(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, float* const* buffer_ptr) {
    return static_cast<ResonanceAudioApi*>(handle)->FillPlanarOutputBuffer(num_channels, num_frames, buffer_ptr);
}
bool resonance_fill_planar_output_buffer_i16(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, int16_t* const* buffer_ptr) {
    return static_cast<ResonanceAudioApi*>(handle)->FillPlanarOutputBuffer(num_channels, num_frames, buffer_ptr);
}

void resonance_set_head_position(ResonanceAudioApiHandle handle, float x, float y, float z) {
    static_cast<ResonanceAudioApi*>(handle)->SetHeadPosition(x, y, z);
}
void resonance_set_head_rotation(ResonanceAudioApiHandle handle, float x, float y, float z, float w) {
    static_cast<ResonanceAudioApi*>(handle)->SetHeadRotation(x, y, z, w);
}
void resonance_set_master_volume(ResonanceAudioApiHandle handle, float volume) {
    static_cast<ResonanceAudioApi*>(handle)->SetMasterVolume(volume);
}
void resonance_set_stereo_speaker_mode(ResonanceAudioApiHandle handle, bool enabled) {
    static_cast<ResonanceAudioApi*>(handle)->SetStereoSpeakerMode(enabled);
}

int resonance_create_ambisonic_source(ResonanceAudioApiHandle handle, size_t num_channels) {
    return static_cast<ResonanceAudioApi*>(handle)->CreateAmbisonicSource(num_channels);
}
int resonance_create_stereo_source(ResonanceAudioApiHandle handle, size_t num_channels) {
    return static_cast<ResonanceAudioApi*>(handle)->CreateStereoSource(num_channels);
}
int resonance_create_sound_object_source(ResonanceAudioApiHandle handle, RenderingMode rendering_mode) {
    return static_cast<ResonanceAudioApi*>(handle)->CreateSoundObjectSource(rendering_mode);
}
void resonance_destroy_source(ResonanceAudioApiHandle handle, int source_id) {
    static_cast<ResonanceAudioApi*>(handle)->DestroySource(source_id);
}

void resonance_set_interleaved_buffer_f32(ResonanceAudioApiHandle handle, int source_id, const float* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<ResonanceAudioApi*>(handle)->SetInterleavedBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}
void resonance_set_interleaved_buffer_i16(ResonanceAudioApiHandle handle, int source_id, const int16_t* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<ResonanceAudioApi*>(handle)->SetInterleavedBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}
void resonance_set_planar_buffer_f32(ResonanceAudioApiHandle handle, int source_id, const float* const* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<ResonanceAudioApi*>(handle)->SetPlanarBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}
void resonance_set_planar_buffer_i16(ResonanceAudioApiHandle handle, int source_id, const int16_t* const* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<ResonanceAudioApi*>(handle)->SetPlanarBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}

void resonance_set_source_distance_attenuation(ResonanceAudioApiHandle handle, int source_id, float distance_attenuation) {
    static_cast<ResonanceAudioApi*>(handle)->SetSourceDistanceAttenuation(source_id, distance_attenuation);
}
void resonance_set_source_distance_model(ResonanceAudioApiHandle handle, int source_id, DistanceRolloffModel rolloff, float min_distance, float max_distance) {
    static_cast<ResonanceAudioApi*>(handle)->SetSourceDistanceModel(source_id, rolloff, min_distance, max_distance);
}
void resonance_set_source_position(ResonanceAudioApiHandle handle, int source_id, float x, float y, float z) {
    static_cast<ResonanceAudioApi*>(handle)->SetSourcePosition(source_id, x, y, z);
}
void resonance_set_source_room_effects_gain(ResonanceAudioApiHandle handle, int source_id, float room_effects_gain) {
    static_cast<ResonanceAudioApi*>(handle)->SetSourceRoomEffectsGain(source_id, room_effects_gain);
}
void resonance_set_source_rotation(ResonanceAudioApiHandle handle, int source_id, float x, float y, float z, float w) {
    static_cast<ResonanceAudioApi*>(handle)->SetSourceRotation(source_id, x, y, z, w);
}
void resonance_set_source_volume(ResonanceAudioApiHandle handle, int source_id, float volume) {
    static_cast<ResonanceAudioApi*>(handle)->SetSourceVolume(source_id, volume);
}
void resonance_set_sound_object_directivity(ResonanceAudioApiHandle handle, int source_id, float alpha, float order) {
    static_cast<ResonanceAudioApi*>(handle)->SetSoundObjectDirectivity(source_id, alpha, order);
}
void resonance_set_sound_object_listener_directivity(ResonanceAudioApiHandle handle, int source_id, float alpha, float order) {
    static_cast<ResonanceAudioApi*>(handle)->SetSoundObjectListenerDirectivity(source_id, alpha, order);
}
void resonance_set_sound_object_near_field_effect_gain(ResonanceAudioApiHandle handle, int source_id, float gain) {
    static_cast<ResonanceAudioApi*>(handle)->SetSoundObjectNearFieldEffectGain(source_id, gain);
}
void resonance_set_sound_object_occlusion_intensity(ResonanceAudioApiHandle handle, int source_id, float intensity) {
    static_cast<ResonanceAudioApi*>(handle)->SetSoundObjectOcclusionIntensity(source_id, intensity);
}
void resonance_set_sound_object_spread(ResonanceAudioApiHandle handle, int source_id, float spread_deg) {
    static_cast<ResonanceAudioApi*>(handle)->SetSoundObjectSpread(source_id, spread_deg);
}
void resonance_enable_room_effects(ResonanceAudioApiHandle handle, bool enable) {
    static_cast<ResonanceAudioApi*>(handle)->EnableRoomEffects(enable);
}
void resonance_set_reflection_properties(ResonanceAudioApiHandle handle, const ReflectionProperties* reflection_properties) {
    static_cast<ResonanceAudioApi*>(handle)->SetReflectionProperties(*reflection_properties);
}
void resonance_set_reverb_properties(ResonanceAudioApiHandle handle, const ReverbProperties* reverb_properties) {
    static_cast<ResonanceAudioApi*>(handle)->SetReverbProperties(*reverb_properties);
}

} // extern "C"
