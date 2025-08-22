use resonance_cxx::Api;

pub struct Renderer {
    api: Api,
    num_channels: usize,
    _frames_per_buffer: usize,
    _sample_rate_hz: i32,
}

impl Renderer {
    pub fn new(sample_rate_hz: i32, num_channels: usize, frames_per_buffer: usize) -> Self {
        let api = Api::new(num_channels, frames_per_buffer, sample_rate_hz)
            .expect("failed to create resonance Api");
        Self {
            api,
            num_channels,
            _frames_per_buffer: frames_per_buffer,
            _sample_rate_hz: sample_rate_hz,
        }
    }

    /// Fill interleaved output. `buffer` must be `num_frames * num_channels` long.
    pub fn process_output_interleaved(&mut self, buffer: &mut [f32], num_frames: usize) -> bool {
        self.api
            .fill_interleaved_f32(self.num_channels, num_frames, buffer)
    }

    /// Fill planar output using safe helper that accepts slices per channel.
    /// Caller provides mutable per-channel slices in `channels`.
    pub fn process_output_planar(&mut self, channels: &mut [&mut [f32]]) -> bool {
        self.api.fill_planar_f32(channels)
    }

    pub fn set_listener_position(&mut self, x: f32, y: f32, z: f32) {
        self.api.set_head_position(x, y, z);
    }

    pub fn set_listener_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.api.set_head_rotation(x, y, z, w);
    }

    /// Borrow the underlying `resonance_cxx::Api` for callers who need to make source calls.
    pub(crate) fn api_mut(&mut self) -> &mut resonance_cxx::Api {
        &mut self.api
    }

    /// Create a stereo (or multi-channel) source via the internal Api.
    pub fn create_stereo_source(&mut self, num_channels: usize) -> i32 {
        self.api.create_stereo_source(num_channels)
    }

    /// Set an interleaved f32 buffer for a source. `num_channels` and `num_frames`
    /// must match the shape of `audio`.
    pub fn set_interleaved_buffer_f32(
        &mut self,
        source_id: i32,
        audio: &[f32],
        num_channels: usize,
        num_frames: usize,
    ) {
        self.api
            .set_interleaved_buffer_f32(source_id, audio, num_channels, num_frames);
    }
}
