use crate::Vec3;

/// Minimal Spatialiser stub used by the audio-system crate.
/// Provides only the small API surface required by the rest of the crate.
pub struct Spatialiser {
    listener_pos: Vec3,
}

impl Spatialiser {
    pub fn new() -> Self {
        Self { listener_pos: [0.0, 0.0, 0.0] }
    }

    pub fn set_listener_position(&mut self, pos: Vec3) {
        self.listener_pos = pos;
    }

    /// Very small mono->stereo processor: copy mono into stereo channels and
    /// apply a simple pan depending on source position.x vs listener.x.
    /// Pan is computed from relative X (source.x - listener.x) and
    /// mapped into [-1,1] then converted to left/right gains.
    pub fn process_mono_to_stereo(&self, mono: &[f32], stereo: &mut [f32], pos: Vec3, gain: f32) {
        let frames = mono.len();
        if stereo.len() < frames * 2 { return; }
        // relative X: positive => source is to the right of listener -> pan right
        let rel_x = pos[0] - self.listener_pos[0];
        // scale down the influence so small offsets don't fully pan; clamp to [-1,1]
        let pan = (rel_x / 4.0).clamp(-1.0, 1.0);
        // convert pan [-1,1] to left/right gains in [0,1]
        let left_gain = ((1.0 - pan) * 0.5) * gain;
        let right_gain = ((1.0 + pan) * 0.5) * gain;
        for i in 0..frames {
            let s = mono[i];
            stereo[2*i] = s * left_gain;
            stereo[2*i+1] = s * right_gain;
        }
    }
}
