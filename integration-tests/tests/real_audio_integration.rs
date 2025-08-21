use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use std::env;

// no external WAV dependency needed; tests generate sine waves inline
use resonance_cxx::{Api, ReverbProperties};
use audio_backend::create_audio_backend;

/// Determines whether the real audio tests should be run.
/// This checks for a `RUN_REAL_AUDIO` environment variable.
fn should_run() -> bool {
    env::var("RUN_REAL_AUDIO").is_ok()
}

// previously we read external WAVs; now we generate sine waves in tests.

#[test]
#[ignore]
fn play_stereo_with_reverb() {
    if !should_run() { return; }

    // Use Arc<Mutex<Api>> to share the Api object between the main thread
    // and the audio rendering thread.
    let api = Arc::new(Mutex::new(Api::new(2, 256, 48000).expect("create resonance api")));
    
    // Create the audio backend.
    let mut backend = create_audio_backend().expect("create backend");

    // The audio source will be created and configured on the main thread.
    let src = {
        let mut api_guard = api.lock().unwrap();
        api_guard.create_stereo_source(2)
    };

    // Generate some simple audio data (a 440 Hz sine wave).
    let frames = 48000 / 4;
    let freq = 440.0f32;
    let mut interleaved = vec![0f32; frames * 2];
    for i in 0..frames {
        let t = i as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
        interleaved[i*2] = s;
        interleaved[i*2 + 1] = s;
    }
    
    {
        let mut api_guard = api.lock().unwrap();
        api_guard.set_interleaved_buffer_f32(src, &interleaved, 2, frames);
    }
    
    // Set up some reverb properties.
    let reverb = ReverbProperties { rt60_values: [0.5;9], gain: 0.8 };
    {
        let mut api_guard = api.lock().unwrap();
        api_guard.set_reverb_properties(&reverb);
    }

    // Clone the Arc to pass ownership to the render closure.
    // This ensures the Api object stays alive for as long as the audio thread needs it.
    let api_clone = Arc::clone(&api);
    let render = std::sync::Arc::new(move |buf: &mut [f32], _sr: u32, frames: usize| {
        // The MutexGuard will hold the lock and reference to the Api for the duration of the closure.
        if let Ok(mut api_guard) = api_clone.lock() {
            api_guard.fill_interleaved_f32(2, frames, buf);
        } else {
            // Fill with silence if the lock can't be acquired (e.g., due to a panic on the other thread).
            buf.iter_mut().for_each(|s| *s = 0.0);
        }
    });

    // Start the backend, moving the render closure into the audio thread.
    backend.start(render).expect("start");

    // Play for a few seconds.
    sleep(Duration::from_millis(1500));
    
    // Stop the backend. This signals the audio thread to shut down.
    backend.stop().expect("stop");
    
    // Explicitly drop the backend. This waits for the audio thread to join,
    // ensuring it has fully finished and released its Arc reference.
    std::mem::drop(backend);

    // The test function ends. The original `api` Arc is now the last remaining
    // reference, and it is dropped, safely deallocating the C++ object.
}

#[test]
#[ignore]
fn play_stereo_with_mono_source() {
    if !should_run() { return; }

    let api = Arc::new(Mutex::new(Api::new(2, 256, 48000).expect("create resonance api")));
    let mut backend = create_audio_backend().expect("create backend");
    
    // Create a mono source (use create_stereo_source with 1 channel for a
    // non-spatialized mono source) and stereo output.
    let src = {
        let mut api_guard = api.lock().unwrap();
        api_guard.create_stereo_source(1)
    };
    
    let frames = 48000 / 4;
    let freq = 440.0f32;
    let mut mono_samples = vec![0f32; frames];
    for i in 0..frames {
        let t = i as f32 / 48000.0;
        mono_samples[i] = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
    }
    
    {
        let mut api_guard = api.lock().unwrap();
        // set_planar_buffer_f32 expects a slice of channel slices.
        api_guard.set_planar_buffer_f32(src, &[&mono_samples[..]], frames);
    }
    
    let api_clone = Arc::clone(&api);
    let render = std::sync::Arc::new(move |buf: &mut [f32], _sr: u32, frames: usize| {
        if let Ok(mut api_guard) = api_clone.lock() {
            api_guard.fill_interleaved_f32(2, frames, buf);
        } else {
            buf.iter_mut().for_each(|s| *s = 0.0);
        }
    });

    backend.start(render).expect("start");
    
    sleep(Duration::from_millis(1500));
    
    backend.stop().expect("stop");
    std::mem::drop(backend);
}

#[test]
#[ignore]
fn play_real_audio() {
    if !should_run() { return; }
    
    let api = Arc::new(Mutex::new(Api::new(2, 256, 48000).expect("create resonance api")));
    let mut backend = create_audio_backend().expect("create backend");
    
    // Generate a stereo sine wave buffer and use it as the source data so the
    // test doesn't rely on external audio files.
    let sample_rate = 48000u32;
    let channels = 2usize;
    let duration_secs = 2.0f32;
    let frames = (sample_rate as f32 * duration_secs) as usize;
    let freq = 440.0f32;

    // Interleaved stereo buffer
    let mut samples = vec![0f32; frames * channels];
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let s = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
        samples[i * 2] = s;
        samples[i * 2 + 1] = s;
    }

    let src = {
        let mut api_guard = api.lock().unwrap();
        api_guard.create_stereo_source(2)
    };

    {
        let mut api_guard = api.lock().unwrap();
        api_guard.set_interleaved_buffer_f32(src, &samples, 2, frames);
    }
    
    let api_clone = Arc::clone(&api);
    let render = std::sync::Arc::new(move |buf: &mut [f32], _sr: u32, frames: usize| {
        if let Ok(mut api_guard) = api_clone.lock() {
            api_guard.fill_interleaved_f32(2, frames, buf);
        } else {
            buf.iter_mut().for_each(|s| *s = 0.0);
        }
    });

    backend.start(render).expect("start");
    
    // Play for the duration of the generated buffer.
    let duration_ms = (frames as f32 / sample_rate as f32) * 1000.0;
    sleep(Duration::from_millis(duration_ms as u64));
    
    backend.stop().expect("stop");
    std::mem::drop(backend);
}
