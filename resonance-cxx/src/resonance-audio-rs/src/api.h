// Minimal header to expose the underlying ResonanceAudio types to cxx
#pragma once

// Include the authoritative header from the resonance-audio project that
// declares vraudio::ResonanceAudioApi and related types.
#include <resonance_audio_api.h>
#include "rust/cxx.h"
#include <cstddef>
#include <cstdint>
#include <memory>
#include <array>

// Provide a small compatibility layer in the `ra` namespace so the
// cxx-generated headers can refer to `ra::ResonanceAudioApi` and
// `ra::SourceId`.
namespace ra {
	// Forward declarations for PODs and enums that the cxx bridge will
	// define in the generated header. We declare them here so method
	// signatures can refer to them; their full definitions are provided by
	// cxx in `bridge.rs.h` which includes this file.
	struct ReflectionProperties;
	struct ReverbProperties;
	enum class RenderingMode : ::std::int32_t;
	enum class DistanceRolloffModel : ::std::int32_t;

		using SourceId = std::int32_t;

	// A thin wrapper type visible to the cxx-generated code. We only declare
	// the API surface here (constructors/destructors and member functions).
	// Implementations live in `cxx/src/resonance_api_wrapper.cc` where the
	// generated `ra::ReflectionProperties` and `ra::ReverbProperties` are
	// available for field-by-field conversion to the upstream types.
	class ResonanceAudioApi {
	public:
		explicit ResonanceAudioApi(std::unique_ptr<::vraudio::ResonanceAudioApi> impl);
		~ResonanceAudioApi();

	// Output filling - use uniquely named methods to avoid C++ overload
	// Use rust::Slice<> for interleaved buffers so the cxx bridge generates
	// the expected rust::Slice parameter types in the generated C++.
	bool FillInterleavedOutputBufferF32(size_t num_channels, size_t num_frames, ::rust::Slice<float> buffer);
	bool FillInterleavedOutputBufferI16(size_t num_channels, size_t num_frames, ::rust::Slice<int16_t> buffer);
	bool FillPlanarOutputBufferF32(size_t num_channels, size_t num_frames, float* const* buffer_ptr);
	bool FillPlanarOutputBufferI16(size_t num_channels, size_t num_frames, int16_t* const* buffer_ptr);

		// Head / global
		void SetHeadPosition(float x, float y, float z);
		void SetHeadRotation(float x, float y, float z, float w);
		void SetMasterVolume(float volume);
		void SetStereoSpeakerMode(bool enabled);

		// Sources
		SourceId CreateAmbisonicSource(size_t num_channels);
		SourceId CreateStereoSource(size_t num_channels);
		// Use the ra::RenderingMode type (the cxx-generated enum) so member function
		// pointer types match exactly what the bridge expects.
		SourceId CreateSoundObjectSource(ra::RenderingMode mode);
		void DestroySource(SourceId id);

	// Accept rust::Slice<const T> for immutable audio buffers coming from Rust.
	void SetInterleavedBufferF32(SourceId source_id, ::rust::Slice<const float> audio, size_t num_channels, size_t num_frames);
	void SetInterleavedBufferI16(SourceId source_id, ::rust::Slice<const int16_t> audio, size_t num_channels, size_t num_frames);

	void SetPlanarBufferF32(SourceId source_id, const float* const* audio_buffer_ptr, size_t num_channels, size_t num_frames);
	void SetPlanarBufferI16(SourceId source_id, const int16_t* const* audio_buffer_ptr, size_t num_channels, size_t num_frames);

		void SetSourceDistanceAttenuation(SourceId source_id, float distance_attenuation);
		void SetSourceDistanceModel(SourceId source_id, ra::DistanceRolloffModel rolloff, float min_distance, float max_distance);
		void SetSourcePosition(SourceId source_id, float x, float y, float z);
		void SetSourceRoomEffectsGain(SourceId source_id, float room_effects_gain);
		void SetSourceRotation(SourceId source_id, float x, float y, float z, float w);
		void SetSourceVolume(SourceId source_id, float volume);
		void SetSoundObjectDirectivity(SourceId source_id, float alpha, float order);
		void SetSoundObjectListenerDirectivity(SourceId source_id, float alpha, float order);
		void SetSoundObjectNearFieldEffectGain(SourceId source_id, float gain);
		void SetSoundObjectOcclusionIntensity(SourceId source_id, float intensity);
		void SetSoundObjectSpread(SourceId source_id, float spread_deg);

		void EnableRoomEffects(bool enable);
		void SetReflectionProperties(const ReflectionProperties& p);
		void SetReverbProperties(const ReverbProperties& p);

	private:
		std::unique_ptr<::vraudio::ResonanceAudioApi> impl_;
	};

	// Factory that mirrors the upstream C factory but returns the wrapper
	// type expected by cxx-generated code.
	std::unique_ptr<ResonanceAudioApi> CreateResonanceAudioApi(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz);
	// Do not alias `Api` here; the crate's public wrapper (`cxx/include/resonance_bridge.h`)
	// defines `ra::Api` separately to present a different public surface. Defining
	// `using Api = ResonanceAudioApi` here caused a conflicting redefinition.
}

