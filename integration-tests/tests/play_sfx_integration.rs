use asset_manager::asset_pkg::AssetPkg;
use audio_backend::create_audio_backend;
use resonance_audio_engine::Renderer;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

#[test]
fn play_sfx_from_pkg_or_tone() {
    // Prepare backend and api
    let mut backend = create_audio_backend().expect("create backend");
    let backend_sr = backend.sample_rate();
    let backend_buf = backend.buffer_size();
    let backend_channels = backend.channels() as usize;
    let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };

    // Require real (non-mock) backend for this audible test.
    assert!(
        !audio_backend::is_mock_backend_enabled(),
        "Test requires real OS audio backend (not mock)"
    );

    // Use resonance-audio-engine's Renderer wrapper for simpler output processing.
    let renderer = Arc::new(Mutex::new(Renderer::new(
        backend_sr as i32,
        backend_channels,
        frames_per_buffer,
    )));

    // Try to open the package and find a .sfx asset. Prefer the bundled test.pkg in this crate's tests folder.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("tests").join("test.pkg");
    let pkg_path_buf = if candidate.exists() {
        candidate
    } else {
        Path::new("assets/dest/out.pkg").to_path_buf()
    };
    // We'll load multiple sfx entries below; ensure package exists
    assert!(
        pkg_path_buf.exists(),
        "Package missing at {:?}",
        pkg_path_buf
    );

    // Load up to 3 .sfx assets and create a source for each. We'll stagger their buffer setting
    // so they play overlapping. We'll also take the first asset and loop it repeatedly to play
    // longer audio.
    let mut sfx_entries: Vec<String> = Vec::new();
    if let Ok(pkg) = AssetPkg::open(&pkg_path_buf) {
        for name in pkg.list_names() {
            if name.ends_with(".sfx") {
                sfx_entries.push(name);
            }
        }
    }

    assert!(!sfx_entries.is_empty(), "No .sfx assets in package");

    let play_count = usize::min(3, sfx_entries.len());
    let mut source_ids: Vec<i32> = Vec::new();
    let mut sfx_buffers: Vec<(Vec<f32>, usize)> = Vec::new(); // (interleaved, frames)

    // Read and convert each sfx to backend channels
    let pkg = AssetPkg::open(&pkg_path_buf).expect("open pkg");
    for name in sfx_entries.iter().take(play_count) {
        let (samples, meta) = pkg.read_sfx_samples(name).expect("read sfx");
        let src_ch = meta.channels as usize;
        let src_frames = if src_ch > 0 {
            samples.len() / src_ch
        } else {
            0
        };
        let mut converted: Vec<f32> = Vec::new();
        if src_ch == backend_channels {
            converted = samples;
        } else if src_ch == 1 && backend_channels == 2 {
            converted = Vec::with_capacity(src_frames * 2);
            for s in samples.iter().step_by(src_ch) {
                converted.push(*s);
                converted.push(*s);
            }
        } else if src_ch == 2 && backend_channels == 1 {
            converted = Vec::with_capacity(src_frames);
            for chunk in samples.chunks(2) {
                match chunk {
                    [a, b] => converted.push((a + b) * 0.5),
                    [a] => converted.push(*a),
                    _ => converted.push(0.0),
                }
            }
        } else {
            let possible_frames = samples.len() / backend_channels;
            if possible_frames > 0 {
                converted = samples
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
        }
        let frames_for_buffer = if backend_channels > 0 {
            converted.len() / backend_channels
        } else {
            0
        };
        sfx_buffers.push((converted, frames_for_buffer));
    }

    // Create sources as Renderer slots and keep slot ids; we'll use the command
    // queue to create sources and play buffers from the game thread.
    for (_buf, _frames_n) in sfx_buffers.iter() {
        let slot = {
            let mut r = renderer.lock().unwrap();
            r.alloc_slot().expect("no free slot")
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
        source_ids.push(slot as i32);
    }

    // Render closure will call Renderer.process_output_interleaved to fill output.
    // We need a Mutex-wrapped Renderer; reconstruct a Renderer guard each callback by
    // borrowing via the Api reference (the Renderer was partially moved above). To keep
    // this test straightforward, we'll recreate a small temporary Renderer that wraps
    // the existing Api by calling Api::fill_interleaved_f32 directly via a lock.
    let renderer_for_render = Arc::clone(&renderer);
    let render = Arc::new(move |buf: &mut [f32], _sr: u32, frames_n: usize| {
        if let Ok(mut r) = renderer_for_render.try_lock() {
            let _ = r.process_output_interleaved(buf, frames_n);
        } else {
            for s in buf.iter_mut() {
                *s = 0.0;
            }
        }
    });

    backend.start(render).expect("start");

    // Staggered playback: set buffers for each source at offsets so they overlap.
    // Each step we set the next source's buffer and wait a short time.
    for (i, (buf, _frames_n)) in sfx_buffers.iter().enumerate() {
        let slot = source_ids[i] as usize;
        {
            let r = renderer.lock().unwrap();
            let sender = r.command_sender();
            // PlaySfx will transfer the buffer and start the voice
            let meta = asset_manager::sfx_loader::SfxMetadata {
                channels: backend_channels as u16,
                sample_rate: backend_sr as u32,
                loop_points: None,
            };
            let sfx_buffer = resonance_audio_engine::renderer::SfxBuffer {
                samples: std::sync::Arc::new(buf.clone()),
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
        // wait to allow partial overlap (100..300ms)
        sleep(Duration::from_millis(150));
    }

    // Now play longer audio by repeatedly re-setting the first source's buffer for ~2 seconds
    let loop_src = source_ids[0];
    let (loop_buf, _loop_frames) = &sfx_buffers[0];
    let loops = 12; // depending on buffer length this yields a few seconds
    for _ in 0..loops {
        {
            let r = renderer.lock().unwrap();
            let sender = r.command_sender();
            let meta = asset_manager::sfx_loader::SfxMetadata {
                channels: backend_channels as u16,
                sample_rate: backend_sr as u32,
                loop_points: None,
            };
            let sfx_buffer = resonance_audio_engine::renderer::SfxBuffer {
                samples: std::sync::Arc::new(loop_buf.clone()),
                meta,
            };
            sender
                .push(resonance_audio_engine::renderer::Command::PlaySfx {
                    slot: loop_src as usize,
                    buffer: sfx_buffer,
                    gain: 1.0,
                    pos: None,
                })
                .ok();
        }
        sleep(Duration::from_millis(200));
    }

    backend.stop().expect("stop");
}
