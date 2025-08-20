# resonance-cxx — Usage examples

This crate provides safe Rust helpers over the C++ `ResonanceAudioApi` via `cxx`.

Quick example: reuse a scratch buffer in a real audio callback to avoid allocations

```rust
use resonance_cxx::Api;

// Create the API (for example in initialization)
let mut api = Api::new(2, 64, 48000).expect("failed to create Api");

// Preallocate a scratch buffer and reuse it each audio frame
let mut scratch: Vec<f32> = Vec::with_capacity(1024);

// Inside the audio callback (per-frame):
fn audio_callback(api: &mut Api, scratch: &mut Vec<f32>, output: &mut [f32], num_channels: usize, num_frames: usize) {
    // Example: fill into an interleaved temporary using the scratch
    // (the crate provides helpers to accept the scratch)
    let src_id = api.create_stereo_source(num_channels);
    if src_id >= 0 {
        // prepare planar input slices (example) and set with scratch
        // channels: &[&[f32]] should be prepared by application
        // api.set_planar_buffer_f32_with_scratch(src_id, channels, num_frames, scratch);
    }

    // For output, convert to a planar representation if needed:
    // let mut ch0 = &mut output[0..num_frames];
    // let mut ch1 = &mut output[num_frames..2*num_frames];
    // let mut planar = vec![ch0, ch1];
    // api.fill_planar_f32(&mut planar[..]);
}
```

Notes
- Use the `_with_scratch` helpers to avoid per-frame allocations: keep a `Vec<T>` and pass it in each frame.
- Current helpers are safe and will resize the scratch vector as needed. If you need maximum realtime performance and want to avoid zero-initialization, we can add an `unsafe` variant that sets length without initializing memory.

See `tests/` for working examples and tests covering planar helpers, scratch reuse, and edge-cases.
