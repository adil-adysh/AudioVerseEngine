#include "resonance_c_api.h"
#include "../resonance-audio/resonance_audio/api/resonance_audio_api.h"

// Avoid pulling vraudio symbols into the global namespace to prevent name
// collisions with C wrapper types. Convert between C wrapper types and the
// vraudio types explicitly.

extern "C" {

ResonanceAudioApiHandle resonance_create_api(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz) {
    // Call the fully-qualified factory function from the vraudio namespace.
    vraudio::ResonanceAudioApi* api = vraudio::CreateResonanceAudioApi(num_channels, frames_per_buffer, sample_rate_hz);
    return static_cast<ResonanceAudioApiHandle>(api);
}

void resonance_destroy_api(ResonanceAudioApiHandle handle) {
    delete static_cast<vraudio::ResonanceAudioApi*>(handle);
}

bool resonance_fill_interleaved_output_buffer_f32(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, float* buffer_ptr) {
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->FillInterleavedOutputBuffer(num_channels, num_frames, buffer_ptr);
}
bool resonance_fill_interleaved_output_buffer_i16(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, int16_t* buffer_ptr) {
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->FillInterleavedOutputBuffer(num_channels, num_frames, buffer_ptr);
}
bool resonance_fill_planar_output_buffer_f32(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, float* const* buffer_ptr) {
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->FillPlanarOutputBuffer(num_channels, num_frames, buffer_ptr);
}
bool resonance_fill_planar_output_buffer_i16(ResonanceAudioApiHandle handle, size_t num_channels, size_t num_frames, int16_t* const* buffer_ptr) {
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->FillPlanarOutputBuffer(num_channels, num_frames, buffer_ptr);
}

void resonance_set_head_position(ResonanceAudioApiHandle handle, float x, float y, float z) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetHeadPosition(x, y, z);
}
void resonance_set_head_rotation(ResonanceAudioApiHandle handle, float x, float y, float z, float w) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetHeadRotation(x, y, z, w);
}
void resonance_set_master_volume(ResonanceAudioApiHandle handle, float volume) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetMasterVolume(volume);
}
void resonance_set_stereo_speaker_mode(ResonanceAudioApiHandle handle, bool enabled) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetStereoSpeakerMode(enabled);
}

int resonance_create_ambisonic_source(ResonanceAudioApiHandle handle, size_t num_channels) {
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->CreateAmbisonicSource(num_channels);
}
int resonance_create_stereo_source(ResonanceAudioApiHandle handle, size_t num_channels) {
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->CreateStereoSource(num_channels);
}
int resonance_create_sound_object_source(ResonanceAudioApiHandle handle, RenderingMode rendering_mode) {
    // Convert C RenderingMode to vraudio::RenderingMode explicitly.
    return static_cast<vraudio::ResonanceAudioApi*>(handle)->CreateSoundObjectSource(static_cast<vraudio::RenderingMode>(rendering_mode));
}
void resonance_destroy_source(ResonanceAudioApiHandle handle, int source_id) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->DestroySource(source_id);
}

void resonance_set_interleaved_buffer_f32(ResonanceAudioApiHandle handle, int source_id, const float* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetInterleavedBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}
void resonance_set_interleaved_buffer_i16(ResonanceAudioApiHandle handle, int source_id, const int16_t* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetInterleavedBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}
void resonance_set_planar_buffer_f32(ResonanceAudioApiHandle handle, int source_id, const float* const* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetPlanarBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}
void resonance_set_planar_buffer_i16(ResonanceAudioApiHandle handle, int source_id, const int16_t* const* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetPlanarBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}

void resonance_set_source_distance_attenuation(ResonanceAudioApiHandle handle, int source_id, float distance_attenuation) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSourceDistanceAttenuation(source_id, distance_attenuation);
}
void resonance_set_source_distance_model(ResonanceAudioApiHandle handle, int source_id, DistanceRolloffModel rolloff, float min_distance, float max_distance) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSourceDistanceModel(source_id, static_cast<vraudio::DistanceRolloffModel>(rolloff), min_distance, max_distance);
}
void resonance_set_source_position(ResonanceAudioApiHandle handle, int source_id, float x, float y, float z) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSourcePosition(source_id, x, y, z);
}
void resonance_set_source_room_effects_gain(ResonanceAudioApiHandle handle, int source_id, float room_effects_gain) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSourceRoomEffectsGain(source_id, room_effects_gain);
}
void resonance_set_source_rotation(ResonanceAudioApiHandle handle, int source_id, float x, float y, float z, float w) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSourceRotation(source_id, x, y, z, w);
}
void resonance_set_source_volume(ResonanceAudioApiHandle handle, int source_id, float volume) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSourceVolume(source_id, volume);
}
void resonance_set_sound_object_directivity(ResonanceAudioApiHandle handle, int source_id, float alpha, float order) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSoundObjectDirectivity(source_id, alpha, order);
}
void resonance_set_sound_object_listener_directivity(ResonanceAudioApiHandle handle, int source_id, float alpha, float order) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSoundObjectListenerDirectivity(source_id, alpha, order);
}
void resonance_set_sound_object_near_field_effect_gain(ResonanceAudioApiHandle handle, int source_id, float gain) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSoundObjectNearFieldEffectGain(source_id, gain);
}
void resonance_set_sound_object_occlusion_intensity(ResonanceAudioApiHandle handle, int source_id, float intensity) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSoundObjectOcclusionIntensity(source_id, intensity);
}
void resonance_set_sound_object_spread(ResonanceAudioApiHandle handle, int source_id, float spread_deg) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetSoundObjectSpread(source_id, spread_deg);
}
void resonance_enable_room_effects(ResonanceAudioApiHandle handle, bool enable) {
    static_cast<vraudio::ResonanceAudioApi*>(handle)->EnableRoomEffects(enable);
}
void resonance_set_reflection_properties(ResonanceAudioApiHandle handle, const ReflectionProperties* reflection_properties) {
    // Convert C struct to vraudio::ReflectionProperties and forward.
    vraudio::ReflectionProperties rp;
    rp.room_position[0] = reflection_properties->room_position[0];
    rp.room_position[1] = reflection_properties->room_position[1];
    rp.room_position[2] = reflection_properties->room_position[2];
    rp.room_rotation[0] = reflection_properties->room_rotation[0];
    rp.room_rotation[1] = reflection_properties->room_rotation[1];
    rp.room_rotation[2] = reflection_properties->room_rotation[2];
    rp.room_rotation[3] = reflection_properties->room_rotation[3];
    rp.room_dimensions[0] = reflection_properties->room_dimensions[0];
    rp.room_dimensions[1] = reflection_properties->room_dimensions[1];
    rp.room_dimensions[2] = reflection_properties->room_dimensions[2];
    rp.cutoff_frequency = reflection_properties->cutoff_frequency;
    for (int i = 0; i < 6; ++i) rp.coefficients[i] = reflection_properties->coefficients[i];
    rp.gain = reflection_properties->gain;
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetReflectionProperties(rp);
}
void resonance_set_reverb_properties(ResonanceAudioApiHandle handle, const ReverbProperties* reverb_properties) {
    vraudio::ReverbProperties rp;
    for (int i = 0; i < 9; ++i) rp.rt60_values[i] = reverb_properties->rt60_values[i];
    rp.gain = reverb_properties->gain;
    static_cast<vraudio::ResonanceAudioApi*>(handle)->SetReverbProperties(rp);
}

} // extern "C"
