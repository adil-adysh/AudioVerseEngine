// Include the cxx-generated bridge header to get RA struct definitions for
// ReflectionProperties and ReverbProperties before implementing wrappers.
#include "resonance-cxx/src/bridge.rs.h"
#include "resonance-audio-rs/src/api.h"
#include <resonance_audio_api.h>

namespace ra {

ResonanceAudioApi::ResonanceAudioApi(std::unique_ptr<::vraudio::ResonanceAudioApi> impl) : impl_(std::move(impl)) {}
ResonanceAudioApi::~ResonanceAudioApi() = default;

bool ResonanceAudioApi::FillInterleavedOutputBuffer(size_t num_channels, size_t num_frames, float* buffer_ptr) {
  return impl_->FillInterleavedOutputBuffer(num_channels, num_frames, buffer_ptr);
}

bool ResonanceAudioApi::FillInterleavedOutputBuffer(size_t num_channels, size_t num_frames, int16_t* buffer_ptr) {
  return impl_->FillInterleavedOutputBuffer(num_channels, num_frames, buffer_ptr);
}

bool ResonanceAudioApi::FillPlanarOutputBuffer(size_t num_channels, size_t num_frames, float* const* buffer_ptr) {
  return impl_->FillPlanarOutputBuffer(num_channels, num_frames, buffer_ptr);
}

bool ResonanceAudioApi::FillPlanarOutputBuffer(size_t num_channels, size_t num_frames, int16_t* const* buffer_ptr) {
  return impl_->FillPlanarOutputBuffer(num_channels, num_frames, buffer_ptr);
}

void ResonanceAudioApi::SetHeadPosition(float x, float y, float z) { impl_->SetHeadPosition(x, y, z); }
void ResonanceAudioApi::SetHeadRotation(float x, float y, float z, float w) { impl_->SetHeadRotation(x, y, z, w); }
void ResonanceAudioApi::SetMasterVolume(float volume) { impl_->SetMasterVolume(volume); }
void ResonanceAudioApi::SetStereoSpeakerMode(bool enabled) { impl_->SetStereoSpeakerMode(enabled); }

SourceId ResonanceAudioApi::CreateAmbisonicSource(size_t num_channels) { return impl_->CreateAmbisonicSource(num_channels); }
SourceId ResonanceAudioApi::CreateStereoSource(size_t num_channels) { return impl_->CreateStereoSource(num_channels); }
SourceId ResonanceAudioApi::CreateSoundObjectSource(::vraudio::RenderingMode mode) { return impl_->CreateSoundObjectSource(mode); }
void ResonanceAudioApi::DestroySource(SourceId id) { impl_->DestroySource(id); }

void ResonanceAudioApi::SetInterleavedBuffer(SourceId source_id, const float* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
  impl_->SetInterleavedBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}

void ResonanceAudioApi::SetInterleavedBuffer(SourceId source_id, const int16_t* audio_buffer_ptr, size_t num_channels, size_t num_frames) {
  impl_->SetInterleavedBuffer(source_id, audio_buffer_ptr, num_channels, num_frames);
}

void ResonanceAudioApi::SetSourceDistanceAttenuation(SourceId source_id, float distance_attenuation) { impl_->SetSourceDistanceAttenuation(source_id, distance_attenuation); }
void ResonanceAudioApi::SetSourceDistanceModel(SourceId source_id, ::vraudio::DistanceRolloffModel rolloff, float min_distance, float max_distance) { impl_->SetSourceDistanceModel(source_id, rolloff, min_distance, max_distance); }
void ResonanceAudioApi::SetSourcePosition(SourceId source_id, float x, float y, float z) { impl_->SetSourcePosition(source_id, x, y, z); }
void ResonanceAudioApi::SetSourceRoomEffectsGain(SourceId source_id, float room_effects_gain) { impl_->SetSourceRoomEffectsGain(source_id, room_effects_gain); }
void ResonanceAudioApi::SetSourceRotation(SourceId source_id, float x, float y, float z, float w) { impl_->SetSourceRotation(source_id, x, y, z, w); }
void ResonanceAudioApi::SetSourceVolume(SourceId source_id, float volume) { impl_->SetSourceVolume(source_id, volume); }
void ResonanceAudioApi::SetSoundObjectDirectivity(SourceId source_id, float alpha, float order) { impl_->SetSoundObjectDirectivity(source_id, alpha, order); }
void ResonanceAudioApi::SetSoundObjectListenerDirectivity(SourceId source_id, float alpha, float order) { impl_->SetSoundObjectListenerDirectivity(source_id, alpha, order); }
void ResonanceAudioApi::SetSoundObjectNearFieldEffectGain(SourceId source_id, float gain) { impl_->SetSoundObjectNearFieldEffectGain(source_id, gain); }
void ResonanceAudioApi::SetSoundObjectOcclusionIntensity(SourceId source_id, float intensity) { impl_->SetSoundObjectOcclusionIntensity(source_id, intensity); }
void ResonanceAudioApi::SetSoundObjectSpread(SourceId source_id, float spread_deg) { impl_->SetSoundObjectSpread(source_id, spread_deg); }

void ResonanceAudioApi::EnableRoomEffects(bool enable) { impl_->EnableRoomEffects(enable); }

void ResonanceAudioApi::SetReflectionProperties(const ReflectionProperties& p) {
  ::vraudio::ReflectionProperties rp;
  rp.room_position[0] = p.room_position[0]; rp.room_position[1] = p.room_position[1]; rp.room_position[2] = p.room_position[2];
  rp.room_rotation[0] = p.room_rotation[0]; rp.room_rotation[1] = p.room_rotation[1]; rp.room_rotation[2] = p.room_rotation[2]; rp.room_rotation[3] = p.room_rotation[3];
  rp.room_dimensions[0] = p.room_dimensions[0]; rp.room_dimensions[1] = p.room_dimensions[1]; rp.room_dimensions[2] = p.room_dimensions[2];
  rp.cutoff_frequency = p.cutoff_frequency;
  for (int i = 0; i < 6; ++i) rp.coefficients[i] = p.coefficients[i];
  rp.gain = p.gain;
  impl_->SetReflectionProperties(rp);
}

void ResonanceAudioApi::SetReverbProperties(const ReverbProperties& p) {
  ::vraudio::ReverbProperties rp;
  for (int i = 0; i < 9; ++i) rp.rt60_values[i] = p.rt60_values[i];
  rp.gain = p.gain;
  impl_->SetReverbProperties(rp);
}

std::unique_ptr<ResonanceAudioApi> CreateResonanceAudioApi(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz) {
  ::vraudio::ResonanceAudioApi* raw = ::vraudio::CreateResonanceAudioApi(num_channels, frames_per_buffer, sample_rate_hz);
  if (!raw) return nullptr;
  return std::unique_ptr<ResonanceAudioApi>(new ResonanceAudioApi(std::unique_ptr<::vraudio::ResonanceAudioApi>(raw)));
}

} // namespace ra
