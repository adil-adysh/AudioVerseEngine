#![cfg(feature = "real-audio-tests")]

use asset_manager::asset_pkg::AssetPkg;
use audio_backend::create_audio_backend;
use resonance_audio_engine::{Renderer, Spatializer};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

// This test is ignored by default and intended to be run locally using
// `scripts/run-real-audio-tests.ps1` which enables the `real-audio-tests`
// feature and runs ignored tests. It exercises the real OS audio path and
// uses the Renderer and Spatializer to play a short tone via the audio backend.
#[test]
#[ignore]
fn real_audio_play_tone_with_spatializer() {
    // Create the real backend (CPAL)
    let mut backend = create_audio_backend().expect("create backend");
    let backend_sr = backend.sample_rate();
    let backend_buf = backend.buffer_size();
    let backend_channels = backend.channels() as usize;
    let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };

    // Create a renderer using the backend configuration
    let renderer = Arc::new(Mutex::new(Renderer::new(
        backend_sr as i32,
        backend_channels,
        frames_per_buffer,
    )));

    // Create a slot and corresponding spatializer (we'll use Renderer API to create source)
    let slot = {
        let mut r = renderer.lock().unwrap();
        r.alloc_slot().expect("no slot")
    };
    {
        let r = renderer.lock().unwrap();
        let sender = r.command_sender();
        sender
            .push(resonance_audio_engine::renderer::Command::CreateSource {
                slot,
                mode: resonance_cxx::RenderingMode::kStereoPanning,
            })
            .ok();
    }

    // Render callback: borrow renderer mutex and call process_output_interleaved
    let renderer_for_render = Arc::clone(&renderer);
    let render_cb = Arc::new(move |buf: &mut [f32], _sr: u32, frames_n: usize| {
        if let Ok(mut r) = renderer_for_render.try_lock() {
            let _ = r.process_output_interleaved(buf, frames_n);
        } else {
            for v in buf.iter_mut() {
                *v = 0.0;
            }
        }
    });

    backend.start(render_cb).expect("start backend");

    // Try to find a .sfx asset from the package; if none found fall back to a generated tone.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("tests").join("test.pkg");
    let pkg_path = if candidate.exists() {
        candidate
    } else {
        std::path::Path::new("assets/dest/out.pkg").to_path_buf()
    };

    let mut samples: Vec<f32> = Vec::new();
    let mut frames_for_buffer = 0usize;

    if pkg_path.exists() {
        if let Ok(pkg) = AssetPkg::open(&pkg_path) {
            if let Some(name) = pkg.list_names().into_iter().find(|n| n.ends_with(".sfx")) {
                if let Ok((samps, meta)) = pkg.read_sfx_samples(&name) {
                    // Validate metadata: non-empty and channels > 0
                    if !samps.is_empty() && meta.channels > 0 {
                        // proceed to convert
                        let src_ch = meta.channels as usize;
                        // If sample rate differs from backend, resample to backend_sr
                        let mut samps = if meta.sample_rate != backend_sr as u32 {
                            asset_manager::sfx_loader::resample_interleaved(
                                &samps,
                                meta.sample_rate,
                                backend_sr as u32,
                                src_ch,
                            )
                        } else {
                            samps
                        };
                        let src_frames = if src_ch > 0 { samps.len() / src_ch } else { 0 };
                        if src_ch == backend_channels {
                            samples = samps;
                        } else if src_ch == 1 && backend_channels == 2 {
                            samples = Vec::with_capacity(src_frames * 2);
                            for s in samps.iter() {
                                samples.push(*s);
                                samples.push(*s);
                            }
                        } else if src_ch == 2 && backend_channels == 1 {
                            samples = Vec::with_capacity(src_frames);
                            for chunk in samps.chunks(2) {
                                match chunk {
                                    [a, b] => samples.push((a + b) * 0.5),
                                    [a] => samples.push(*a),
                                    _ => samples.push(0.0),
                                }
                            }
                        } else {
                            samples = samps
                                .chunks(src_ch)
                                .flat_map(|chunk| {
                                    let mut out = Vec::with_capacity(backend_channels);
                                    for i in 0..backend_channels {
                                        out.push(*chunk.get(i).unwrap_or(&0.0));
                                    }
                                    out
                                })
                                .collect();
                        }
                        frames_for_buffer = if backend_channels > 0 {
                            samples.len() / backend_channels
                        } else {
                            0
                        };
                    } else {
                        eprintln!(
                            "sfx metadata invalid or mismatched; falling back to generated tone"
                        );
                    }
                }
            }
        }
    }

    if samples.is_empty() {
        // Fallback: generate a short sine tone
        let freq = 440.0f32;
        let duration_secs = 3.0f32;
        let total_frames = (backend_sr as f32 * duration_secs) as usize;
        samples = Vec::with_capacity(total_frames * backend_channels);
        for n in 0..total_frames {
            let t = n as f32 / backend_sr as f32;
            let s = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.2;
            for _ch in 0..backend_channels {
                samples.push(s);
            }
        }
        frames_for_buffer = total_frames;
    }

    // Play the prepared samples via the Renderer by sending a PlaySfx command.
    // This transfers ownership of the buffer into the renderer voice and starts playback
    // without holding the renderer lock while the backend runs.
    {
        let r = renderer.lock().unwrap();
        let sender = r.command_sender();
        let meta = asset_manager::sfx_loader::SfxMetadata {
            channels: backend_channels as u16,
            sample_rate: backend_sr as u32,
            loop_points: None,
        };
        let sfx_buffer = resonance_audio_engine::renderer::SfxBuffer {
            samples: std::sync::Arc::new(samples.clone()),
            meta,
        };
        sender
            .push(resonance_audio_engine::renderer::Command::PlaySfx {
                slot,
                buffer: sfx_buffer,
                gain: 1.0,
                pos: None,
            })
            .ok();
    }

    // Let it play for a few seconds
    sleep(Duration::from_secs(4));

    backend.stop().expect("stop");
}
