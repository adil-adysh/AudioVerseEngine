use std::f32::consts::PI;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::sleep;
use std::time::Duration;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};

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

    // Play a sine tone for a few seconds using the default config.
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let duration_secs = 3.0f32;
    let freq = 440.0f32;

    let playing = Arc::new(AtomicBool::new(true));
    let playing_cloned = playing.clone();

    let err_fn = |err| eprintln!("Stream error: {:?}", err);

    let mut sample_clock = 0f32;
    let two_pi = 2.0 * PI;

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(channels) {
                        let value = (two_pi * freq * sample_clock / sample_rate).sin() * 0.2;
                        sample_clock = (sample_clock + 1.0) % sample_rate;
                        for sample in frame.iter_mut() {
                            *sample = value;
                        }
                    }
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
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [i16], _| {
                    for frame in data.chunks_mut(channels) {
                        let value_f = (two_pi * freq * sample_clock / sample_rate).sin() * 0.2;
                        sample_clock = (sample_clock + 1.0) % sample_rate;
                        let value_i = (value_f * i16::MAX as f32) as i16;
                        for sample in frame.iter_mut() {
                            *sample = value_i;
                        }
                    }
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
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [u16], _| {
                    for frame in data.chunks_mut(channels) {
                        let value_f = (two_pi * freq * sample_clock / sample_rate).sin() * 0.2;
                        sample_clock = (sample_clock + 1.0) % sample_rate;
                        let value_u = ((value_f * 0.5 + 0.5) * u16::MAX as f32) as u16;
                        for sample in frame.iter_mut() {
                            *sample = value_u;
                        }
                    }
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

    sleep(Duration::from_secs_f32(duration_secs));

    println!("Stopping...");
    playing_cloned.store(false, Ordering::SeqCst);
    // allow stream to drop and stop
    drop(stream);
    println!("Done.");
}
