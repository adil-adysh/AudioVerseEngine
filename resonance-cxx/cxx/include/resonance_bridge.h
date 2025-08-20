#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
// The cxx-generated header (`bridge.rs.h`) will include this file; do not
// include the generated header here to avoid circular includes and
// redeclaration issues.

// Include the authoritative resonance audio header so the C++ generated code
// and the wrapper's unique_ptr members have a complete type for
// `vraudio::ResonanceAudioApi`.
// Forward-declare the minimal vraudio API here so this header stays small and
// easy for cxx to parse. The full `resonance_audio_api.h` is included by the
// implementation file so the destructor can be defined with a complete type.
namespace vraudio {
  class ResonanceAudioApi;
  extern "C" ResonanceAudioApi* CreateResonanceAudioApi(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz);
}

// Forward declarations for types that the cxx-generated header will provide
namespace ra {
  // Forward-declare the scoped enums and structs the cxx-generated header
  // will later define. Use the same `enum class`/types to avoid redeclaration
  // and pointer-to-member mismatches.
  enum class RenderingMode : ::std::int32_t;
  enum class DistanceRolloffModel : ::std::int32_t;
  struct ReflectionProperties;
  struct ReverbProperties;

class Api {
public:
  explicit Api(std::unique_ptr<vraudio::ResonanceAudioApi> impl);
  Api(Api&&) = default;
  Api& operator=(Api&&) = default;
  Api(const Api&) = delete;
  Api& operator=(const Api&) = delete;
  ~Api();

  vraudio::ResonanceAudioApi* get() { return impl_.get(); }
  // Member wrappers expected by cxx (these forward to the underlying impl_)
  bool fill_interleaved_f32(std::size_t num_channels, std::size_t num_frames, rust::Slice<float> buffer);
  bool fill_interleaved_i16(std::size_t num_channels, std::size_t num_frames, rust::Slice<int16_t> buffer);

  void set_head_position(float x, float y, float z);
  void set_head_rotation(float x, float y, float z, float w);
  void set_master_volume(float volume);
  void set_stereo_speaker_mode(bool enabled);

  int create_ambisonic_source(std::size_t num_channels);
  int create_stereo_source(std::size_t num_channels);
  int create_sound_object_source(ra::RenderingMode rendering_mode);
  void destroy_source(int source_id);

  void set_interleaved_buffer_f32(int source_id, rust::Slice<const float> audio, std::size_t num_channels, std::size_t num_frames);
  void set_interleaved_buffer_i16(int source_id, rust::Slice<const int16_t> audio, std::size_t num_channels, std::size_t num_frames);

  void set_source_distance_attenuation(int source_id, float distance_attenuation);
  void set_source_distance_model(int source_id, ra::DistanceRolloffModel rolloff, float min_distance, float max_distance);
  void set_source_position(int source_id, float x, float y, float z);
  void set_source_room_effects_gain(int source_id, float room_effects_gain);
  void set_source_rotation(int source_id, float x, float y, float z, float w);
  void set_source_volume(int source_id, float volume);
  void set_sound_object_directivity(int source_id, float alpha, float order);
  void set_sound_object_listener_directivity(int source_id, float alpha, float order);
  void set_sound_object_near_field_effect_gain(int source_id, float gain);
  void set_sound_object_occlusion_intensity(int source_id, float intensity);
  void set_sound_object_spread(int source_id, float spread_deg);

  void enable_room_effects(bool enable);
  void set_reflection_properties(const ra::ReflectionProperties& props);
  void set_reverb_properties(const ra::ReverbProperties& props);

private:
  std::unique_ptr<vraudio::ResonanceAudioApi> impl_;
};

std::unique_ptr<Api> make_api(std::size_t num_channels, std::size_t frames_per_buffer, int sample_rate_hz);

// The generated C++ header provides the shared POD definitions (ReflectionProperties,
// ReverbProperties, RenderingMode, DistanceRolloffModel) and will include this
// header via `include!("resonance_bridge.h")` in the bridge. We implement member
// methods on `ra::Api` above to match the Rust `self: Pin<&mut Api>` declarations.

} // namespace ra
