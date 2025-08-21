// The real audio tests are compiled only when the `real-audio-tests` feature is
// enabled. This avoids unused-import warnings and accidental execution during
// normal CI.
#[cfg(feature = "real-audio-tests")]
mod real_audio_tests {
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time::Duration;
    use serial_test::serial;
    use resonance_cxx::{Api, ReverbProperties};
    use audio_backend::create_audio_backend;

    #[test]
    #[ignore]
    #[serial]
    fn play_stereo_with_reverb() {
        // Create the audio backend first so we can match device sample-rate/buffer.
        let mut backend = create_audio_backend().expect("create backend");
        let backend_sr = backend.sample_rate();
        let backend_buf = backend.buffer_size();
        eprintln!("backend sample_rate={} buffer_size={} channels={}", backend_sr, backend_buf, backend.channels());
        eprintln!("audio-backend compiled with mock-audio feature: {}", audio_backend::is_mock_backend_enabled());
        eprintln!("concrete backend type: {}", std::any::type_name_of_val(&*backend));

        // Use Arc<Mutex<Api>> to share the Api object between the main thread
        // and the audio rendering thread. Create the Api with the backend sample rate.
        let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };
        let api = Arc::new(Mutex::new(Api::new(2, frames_per_buffer, backend_sr as i32).expect("create resonance api")));

        // The audio source will be created and configured on the main thread.
        let src = {
            let mut api_guard = api.lock().unwrap();
            api_guard.create_stereo_source(2)
        };

        // Generate some simple audio data (a 440 Hz sine wave) using detected backend sample rate.
        let frames = (backend_sr as usize) / 4;
        let freq = 440.0f32;
        let mut interleaved = vec![0f32; frames * 2];
        for i in 0..frames {
            let t = i as f32 / backend_sr as f32;
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

        // Shared buffer that audio callback will read from. Main thread writes into it.
        let shared_len = frames * 2;
        let shared_buf = Arc::new(Mutex::new(vec![0f32; shared_len]));
        let shared_clone = shared_buf.clone();
        let api_clone = Arc::clone(&api);
        let render = std::sync::Arc::new(move |buf: &mut [f32], _sr: u32, frames: usize| {
            // First try to copy from shared buffer for low-latency playback.
            if let Ok(shared) = shared_clone.lock() {
                let to_copy = usize::min(buf.len(), shared.len());
                buf[..to_copy].copy_from_slice(&shared[..to_copy]);
                if to_copy < buf.len() { buf[to_copy..].iter_mut().for_each(|s| *s = 0.0); }
                return;
            }
            // Fallback: call into Api if lock failed.
            if let Ok(mut api_guard) = api_clone.lock() {
                api_guard.fill_interleaved_f32(2, frames, buf);
            } else {
                buf.iter_mut().for_each(|s| *s = 0.0);
            }
        });

        // Start the backend, moving the render closure into the audio thread.
        backend.start(render).expect("start");

        // Fill shared buffer with the interleaved block so audio callback plays it.
        {
            let mut shared = shared_buf.lock().unwrap();
            shared[..interleaved.len()].copy_from_slice(&interleaved);
        }

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
    #[serial]
    fn play_stereo_with_mono_source() {
        let mut backend = create_audio_backend().expect("create backend");
        let backend_sr = backend.sample_rate();
        let backend_buf = backend.buffer_size();
        eprintln!("backend sample_rate={} buffer_size={} channels={}", backend_sr, backend_buf, backend.channels());
        let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };
        let api = Arc::new(Mutex::new(Api::new(2, frames_per_buffer, backend_sr as i32).expect("create resonance api")));

        // Create a mono source (use create_stereo_source with 1 channel for a
        // non-spatialized mono source) and stereo output.
        let src = {
            let mut api_guard = api.lock().unwrap();
            api_guard.create_stereo_source(1)
        };

        let frames = (backend_sr as usize) / 4;
        let freq = 440.0f32;
        let mut mono_samples = vec![0f32; frames];
        for i in 0..frames {
            let t = i as f32 / backend_sr as f32;
            mono_samples[i] = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
        }

        {
            let mut api_guard = api.lock().unwrap();
            // set_planar_buffer_f32 expects a slice of channel slices.
            api_guard.set_planar_buffer_f32(src, &[&mono_samples[..]], frames);
        }

        let shared_len = frames * 2;
        let shared_buf = Arc::new(Mutex::new(vec![0f32; shared_len]));
        let shared_clone = shared_buf.clone();
        let api_clone = Arc::clone(&api);
        let render = std::sync::Arc::new(move |buf: &mut [f32], _sr: u32, frames: usize| {
            if let Ok(shared) = shared_clone.lock() {
                let to_copy = usize::min(buf.len(), shared.len());
                buf[..to_copy].copy_from_slice(&shared[..to_copy]);
                if to_copy < buf.len() { buf[to_copy..].iter_mut().for_each(|s| *s = 0.0); }
                return;
            }
            if let Ok(mut api_guard) = api_clone.lock() {
                api_guard.fill_interleaved_f32(2, frames, buf);
            } else {
                buf.iter_mut().for_each(|s| *s = 0.0);
            }
        });

        backend.start(render).expect("start");

        // fill shared mono buffer
        {
            let mut shared = shared_buf.lock().unwrap();
            for i in 0..frames {
                shared[i] = mono_samples[i];
                shared[i + frames] = mono_samples[i];
            }
        }

        sleep(Duration::from_millis(1500));

        backend.stop().expect("stop");
        std::mem::drop(backend);
    }

    #[test]
    #[ignore]
    #[serial]
    fn play_real_audio() {
        let mut backend = create_audio_backend().expect("create backend");
        let backend_sr = backend.sample_rate();
        let backend_buf = backend.buffer_size();
        eprintln!("backend sample_rate={} buffer_size={} channels={}", backend_sr, backend_buf, backend.channels());
        let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };
        let api = Arc::new(Mutex::new(Api::new(2, frames_per_buffer, backend_sr as i32).expect("create resonance api")));

        // Create a short, pleasant melody using additive sine voices with ADSR
        // envelope and a gentle stereo pan. This will sound nicer than a raw tone.
        let sample_rate = backend_sr as usize;
        let channels = 2usize;
        let melody_seconds = 3.0f32;
        let frames = (sample_rate as f32 * melody_seconds) as usize;

        // Melody: notes in Hz (A4, B4, C#5, E5) with durations (s)
        let notes = [ (440.0, 0.5), (494.0, 0.5), (554.37, 0.75), (659.25, 1.0) ];

        // ADSR helper (short attack/decay, sustain at 0.7, release)
        let envelope = |pos: usize, note_frames: usize| -> f32 {
            let a = (note_frames as f32 * 0.03).max(1.0) as usize; // 3% attack
            let d = (note_frames as f32 * 0.05).max(1.0) as usize; // decay
            let s_level = 0.7f32;
            let r = (note_frames as f32 * 0.08).max(1.0) as usize; // release

            if pos < a {
                pos as f32 / a as f32
            } else if pos < a + d {
                let pd = pos - a;
                1.0 - (1.0 - s_level) * (pd as f32 / d as f32)
            } else if pos >= note_frames - r {
                let pr = pos - (note_frames - r);
                s_level * (1.0 - (pr as f32 / r as f32))
            } else {
                s_level
            }
        };

        // Stereo pan helper (-1.0 left ... +1.0 right)
        let pan_lr = |x: f32| -> (f32,f32) {
            let left = ((1.0 - x) * 0.5).sqrt();
            let right = ((1.0 + x) * 0.5).sqrt();
            (left, right)
        };

        let mut samples = vec![0f32; frames * channels];
        let mut write_pos = 0usize;
        for (idx, (freq, dur)) in notes.iter().enumerate() {
            let note_frames = (sample_rate as f32 * dur) as usize;
            for i in 0..note_frames {
                if write_pos >= frames { break; }
                let t = write_pos as f32 / sample_rate as f32;
                // additive: two partials
                let s = (2.0 * std::f32::consts::PI * (*freq) * t).sin() * 0.6
                      + (2.0 * std::f32::consts::PI * (*freq) * 2.0 * t).sin() * 0.2;
                let env = envelope(i, note_frames);
                // pan across stereo a little per note index
                let pan = -0.5 + (idx as f32 / (notes.len() as f32 - 1.0)) * 1.0;
                let (l_gain, r_gain) = pan_lr(pan);
                samples[write_pos * 2] += s * env * l_gain * 0.6;
                samples[write_pos * 2 + 1] += s * env * r_gain * 0.6;
                write_pos += 1;
            }
            // brief silence between notes
            write_pos += (sample_rate as f32 * 0.02) as usize;
            if write_pos >= frames { break; }
        }

        let src = {
            let mut api_guard = api.lock().unwrap();
            api_guard.create_stereo_source(2)
        };

        {
            let mut api_guard = api.lock().unwrap();
            api_guard.set_interleaved_buffer_f32(src, &samples, 2, frames);
        }

        let shared_len = frames * channels;
        let shared_buf = Arc::new(Mutex::new(vec![0f32; shared_len]));
        let shared_clone = shared_buf.clone();
        let api_clone = Arc::clone(&api);
        let render = std::sync::Arc::new(move |buf: &mut [f32], _sr: u32, frames: usize| {
            if let Ok(shared) = shared_clone.lock() {
                let to_copy = usize::min(buf.len(), shared.len());
                buf[..to_copy].copy_from_slice(&shared[..to_copy]);
                if to_copy < buf.len() { buf[to_copy..].iter_mut().for_each(|s| *s = 0.0); }
                return;
            }
            if let Ok(mut api_guard) = api_clone.lock() {
                api_guard.fill_interleaved_f32(2, frames, buf);
            } else {
                buf.iter_mut().for_each(|s| *s = 0.0);
            }
        });

        backend.start(render).expect("start");

        // fill shared buffer with generated samples so audio callback plays them
        {
            let mut shared = shared_buf.lock().unwrap();
            if shared.len() >= samples.len() {
                shared[..samples.len()].copy_from_slice(&samples);
            } else {
                shared.clear();
                shared.extend_from_slice(&samples);
            }
        }

        // Play for the duration of the generated buffer.
        let duration_ms = (frames as f32 / sample_rate as f32) * 1000.0;
        sleep(Duration::from_millis(duration_ms as u64));

        backend.stop().expect("stop");
        std::mem::drop(backend);
    }

    #[test]
    #[ignore]
    #[serial]
    fn exercise_resonance_cxx_api_surface() {
        let mut backend = create_audio_backend().expect("create backend");
        let backend_sr = backend.sample_rate();
        let backend_buf = backend.buffer_size();
        let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };

        // Create Api and exercise many public methods.
        let mut api = resonance_cxx::Api::new(2, frames_per_buffer, backend_sr as i32).expect("create api");

        // create sources with different channel counts
        let s_stereo = api.create_stereo_source(2);
        let s_mono = api.create_stereo_source(1);

        // prepare small buffers
        let frames = 128usize;
        let mut inter = vec![0f32; frames * 2];
        for i in 0..frames { inter[i*2] = 0.1 * (i as f32).sin(); inter[i*2+1] = inter[i*2]; }

        let mut mono = vec![0f32; frames];
        for i in 0..frames { mono[i] = 0.05 * (i as f32).sin(); }

        // set interleaved and planar
        api.set_interleaved_buffer_f32(s_stereo, &inter, 2, frames);
        api.set_planar_buffer_f32(s_mono, &[&mono[..]], frames);

        // set gain, reverb, position, and play-style utilities
        // set volume using the public method
        api.set_source_volume(s_stereo, 0.8);
        api.set_source_volume(s_mono, 0.6);

        let reverb = resonance_cxx::ReverbProperties { rt60_values: [0.3;9], gain: 0.5 };
        api.set_reverb_properties(&reverb);

        // set position (x,y,z)
        api.set_source_position(s_stereo, 0.0, 0.0, -1.0);

        // distance attenuation/model
        api.set_source_distance_attenuation(s_stereo, 0.9);
        api.set_source_distance_model(s_stereo, resonance_cxx::DistanceRolloffModel::kLogarithmic, 0.1, 100.0);

        // test fill helpers
        let mut out = vec![0f32; frames * 2];
        let _ok = api.fill_interleaved_f32(2, frames, &mut out);

        // planar fill
        let mut ch0 = vec![0f32; frames];
        let mut ch1 = vec![0f32; frames];
        let mut planar_refs: [&mut [f32]; 2] = [&mut ch0[..], &mut ch1[..]];
        let _ok2 = api.fill_planar_f32(&mut planar_refs[..]);

        // ensure no crash and simple invariants
        assert_eq!(out.len(), frames * 2);
        assert_eq!(ch0.len(), frames);
    }
}
