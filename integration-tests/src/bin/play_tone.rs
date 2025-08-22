use std::f32::consts::PI;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::sleep;
use std::time::Duration;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};

use resonance_cxx::Api;
use std::sync::{Mutex, atomic::AtomicUsize};

fn main() {
    let host = cpal::default_host();

    println!("Hosts available");
    // List devices
    match host.devices() {
        Ok(devices) => {
            for (i, d) in devices.enumerate() {
                let name = d.name().unwrap_or_else(|_| "<unknown>".into());
                println!("Device #{}: {}", i, name);
            }
        }
        Err(e) => println!("Failed to enumerate devices: {:?}", e),
    }

    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            eprintln!("No default output device found");
            return;
        }
    };
    let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());
    println!("Default output device: {}", device_name);

    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to get default output config: {:?}", e);
            return;
        }
    };
    println!("Default config: {:?}", config);

    // We'll create the Resonance Api after we've derived `sample_rate` and
    // `channels` from the device config below so the correct values are in
    // scope.

    // Play a sine tone for a few seconds using the default config.
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let freq = 440.0f32;

    let playing = Arc::new(AtomicBool::new(true));
    let playing_cloned = playing.clone();

    let err_fn = |err| eprintln!("Stream error: {:?}", err);

    let two_pi = 2.0 * PI;

    // Waveform types we want to exercise against the Resonance API.
    #[derive(Copy, Clone)]
    enum Waveform {
        Sine,
        Square,
        Triangle,
        Sawtooth,
    }

    impl Waveform {
        fn samples(&self, t: f32, freq: f32) -> f32 {
            let x = 2.0 * PI * freq * t;
            match self {
                Waveform::Sine => x.sin(),
                Waveform::Square => if x.sin() >= 0.0 { 1.0 } else { -1.0 },
                Waveform::Triangle => {
                    // normalized triangle: range [-1,1]
                    let period = 2.0 * PI;
                    let v = (x % period - PI).abs();
                    (2.0 / PI) * v - 1.0
                }
                Waveform::Sawtooth => {
                    // normalized sawtooth: range [-1,1]
                    let period = 2.0 * PI;
                    2.0 * (x / period - (x / period).floor()) - 1.0
                }
            }
        }
    }

    // shared buffer that audio callback will read from and tests will write to
    let num_frames = (sample_rate as f32 * 0.1) as usize; // 100ms block
    let shared_len = num_frames * channels;
    let shared_buf = std::sync::Arc::new(Mutex::new(vec![0f32; shared_len]));
    let shared_pos = std::sync::Arc::new(AtomicUsize::new(0));

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let shared_buf_cloned = shared_buf.clone();
            let shared_pos_cloned = shared_pos.clone();
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    let buf = shared_buf_cloned.lock().unwrap();
                    let buf_len = buf.len();
                    let mut pos = shared_pos_cloned.load(std::sync::atomic::Ordering::Relaxed);
                    for frame in data.chunks_mut(channels) {
                        if buf_len == 0 {
                            for sample in frame.iter_mut() { *sample = 0.0; }
                            continue;
                        }
                        let base = pos % buf_len;
                        for ch in 0..channels {
                            let v = buf[(base + ch) % buf_len];
                            frame[ch] = v;
                        }
                        pos = (base + channels) % buf_len;
                    }
                    shared_pos_cloned.store(pos, std::sync::atomic::Ordering::Relaxed);
                },
                err_fn,
                None,
            );
            match stream {
                Ok(s) => s,
                Err(e) => { eprintln!("Failed to build stream: {:?}", e); return; }
            }
        }
        cpal::SampleFormat::I16 => {
            let shared_buf_cloned = shared_buf.clone();
            let shared_pos_cloned = shared_pos.clone();
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [i16], _| {
                    let buf = shared_buf_cloned.lock().unwrap();
                    let buf_len = buf.len();
                    let mut pos = shared_pos_cloned.load(std::sync::atomic::Ordering::Relaxed);
                    for frame in data.chunks_mut(channels) {
                        if buf_len == 0 {
                            for sample in frame.iter_mut() { *sample = 0; }
                            continue;
                        }
                        let base = pos % buf_len;
                        for ch in 0..channels {
                            let f = buf[(base + ch) % buf_len];
                            frame[ch] = (f * i16::MAX as f32) as i16;
                        }
                        pos = (base + channels) % buf_len;
                    }
                    shared_pos_cloned.store(pos, std::sync::atomic::Ordering::Relaxed);
                },
                err_fn,
                None,
            );
            match stream {
                Ok(s) => s,
                Err(e) => { eprintln!("Failed to build stream: {:?}", e); return; }
            }
        }
        cpal::SampleFormat::U16 => {
            let shared_buf_cloned = shared_buf.clone();
            let shared_pos_cloned = shared_pos.clone();
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [u16], _| {
                    let buf = shared_buf_cloned.lock().unwrap();
                    let buf_len = buf.len();
                    let mut pos = shared_pos_cloned.load(std::sync::atomic::Ordering::Relaxed);
                    for frame in data.chunks_mut(channels) {
                        if buf_len == 0 {
                            for sample in frame.iter_mut() { *sample = 0; }
                            continue;
                        }
                        let base = pos % buf_len;
                        for ch in 0..channels {
                            let f = buf[(base + ch) % buf_len];
                            frame[ch] = ((f * 0.5 + 0.5) * u16::MAX as f32) as u16;
                        }
                        pos = (base + channels) % buf_len;
                    }
                    shared_pos_cloned.store(pos, std::sync::atomic::Ordering::Relaxed);
                },
                err_fn,
                None,
            );
            match stream {
                Ok(s) => s,
                Err(e) => { eprintln!("Failed to build stream: {:?}", e); return; }
            }
        }
        _ => {
            eprintln!("Unsupported sample format");
            return;
        }
    };

    println!("Starting stream...");
    if let Err(e) = stream.play() {
        eprintln!("Failed to play stream: {:?}", e);
        return;
    }

    // Create a resonance Api instance that we'll use to exercise the FFI while
    // the audio stream is running. Use the device/config values discovered
    // above (channels / sample_rate) so the Api is configured to match.
    let sample_rate_i32 = sample_rate as i32;
    let mut ra_api = match Api::new(channels, 256, sample_rate_i32) {
        Some(a) => a,
        None => { eprintln!("Failed to create Resonance Audio Api"); return; }
    };

    // Create a stereo source that we'll update with generated audio. The
    // num_channels parameter is the source's channel count (stereo -> 2).
    let source_id = ra_api.create_stereo_source(2);

    // While the stream is running, we'll also feed one buffer into the
    // Resonance API to exercise the FFI paths. Create an interleaved buffer
    // matching the stream format (f32 samples) and call the API setter. This
    // demonstrates how to use `set_interleaved_buffer_f32` from tests.
    let num_frames = (sample_rate as f32 * 0.1) as usize; // 100ms buffer
    // Pre-generate buffers for each waveform type so we can cycle them quickly
    let waveforms = [Waveform::Sine, Waveform::Square, Waveform::Triangle, Waveform::Sawtooth];
    let mut interleaved_per_wave: Vec<Vec<f32>> = Vec::with_capacity(waveforms.len());
    for wf in waveforms.iter() {
        let mut buf = vec![0f32; num_frames * channels];
        for frame in 0..num_frames {
            let t = (frame as f32) / sample_rate;
            let v = wf.samples(t, freq) * 0.2;
            for ch in 0..channels {
                buf[frame * channels + ch] = v;
            }
        }
        interleaved_per_wave.push(buf);
    }

    // (we'll set per-wave interleaved buffers in the runtime loop below)

    // Also demonstrate planar buffers: create per-channel slices and call the
    // planar setter. This exercises the scratch/planar helpers on the Rust API.
    let mut left = vec![0f32; num_frames];
    let mut right = vec![0f32; num_frames];
    for i in 0..num_frames {
        let t = (i as f32) / sample_rate;
        left[i] = (two_pi * (freq * 0.5) * t).sin() * 0.15;
        right[i] = (two_pi * (freq * 1.5) * t).sin() * 0.15;
    }
    let left_ref: &[f32] = &left;
    let right_ref: &[f32] = &right;
    let channels_ref: Vec<&[f32]> = vec![left_ref, right_ref];
    // call the planar setter which will interleave and forward to C++.
    ra_api.set_planar_buffer_f32(source_id, channels_ref.as_slice(), num_frames);

    // Apply a few parameter changes while audio plays to exercise other APIs.
    ra_api.set_head_position(0.0, 0.0, 0.0);
    ra_api.set_head_rotation(0.0, 0.0, 0.0, 1.0);

    // Define explicit test cases with distinct audible properties so each
    // produces a unique sound: waveform, frequency, duration, and stereo pan.
    struct TestCase {
        name: &'static str,
        waveform: Waveform,
        freq: f32,
        duration_s: f32,
        pan: f32, // -1.0 = left, 0.0 = center, +1.0 = right
    }

    let test_cases = [
        TestCase { name: "sine-440-center", waveform: Waveform::Sine, freq: 440.0, duration_s: 2.0, pan: 0.0 },
        TestCase { name: "square-220-left", waveform: Waveform::Square, freq: 220.0, duration_s: 2.0, pan: -0.8 },
        TestCase { name: "triangle-660-right", waveform: Waveform::Triangle, freq: 660.0, duration_s: 2.0, pan: 0.8 },
        TestCase { name: "sawtooth-110-center", waveform: Waveform::Sawtooth, freq: 110.0, duration_s: 3.0, pan: 0.0 },
    ];

    // For each test case, generate an interleaved stereo buffer with pan
    // applied and submit it repeatedly to the resonance API for the
    // test's duration so the test is clearly audible.
    for tc in test_cases.iter() {
        println!("Running test: {} ({} Hz, pan={})", tc.name, tc.freq, tc.pan);
        let frames_per_block = num_frames;
    
    let blocks = ((tc.duration_s * sample_rate) as usize).div_ceil(frames_per_block);

        // generate one block for this test, then submit it `blocks` times
        let mut block_buf = vec![0f32; frames_per_block * channels];
        for frame in 0..frames_per_block {
            let t = (frame as f32) / sample_rate;
            let v = tc.waveform.samples(t, tc.freq) * 0.25; // slightly louder
            // simple linear pan
            let left_gain = if tc.pan <= 0.0 { 1.0 } else { 1.0 - tc.pan };
            let right_gain = if tc.pan >= 0.0 { 1.0 } else { 1.0 + tc.pan };
            for ch in 0..channels {
                let sample = if channels == 1 {
                    v
                } else if ch == 0 {
                    v * left_gain
                } else {
                    v * right_gain
                };
                block_buf[frame * channels + ch] = sample;
            }
        }

        // Submit the block repeatedly for the duration of the test case by
        // copying it into the shared buffer that the audio callback reads.
        for _ in 0..blocks {
            {
                let mut shared = shared_buf.lock().unwrap();
                if shared.len() >= block_buf.len() {
                    shared[..block_buf.len()].copy_from_slice(&block_buf);
                } else {
                    // resize and copy if needed
                    shared.clear();
                    shared.extend_from_slice(&block_buf);
                }
            }
            // Also exercise the resonance API setter so both paths are used.
            ra_api.set_interleaved_buffer_f32(source_id, &block_buf, channels, frames_per_block);
            // short sleep to let the audio thread consume the buffer; tune as needed
            sleep(Duration::from_millis((frames_per_block as f32 / sample_rate * 1000.0) as u64));
        }
    }

    println!("Stopping...");
    playing_cloned.store(false, Ordering::SeqCst);
    // allow stream to drop and stop
    drop(stream);
    println!("Done.");
}
