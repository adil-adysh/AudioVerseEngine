use asset_manager::asset_pkg::AssetPkg;
use audio_backend::create_audio_backend;
use resonance_audio_engine::Renderer;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // Find package
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("tests").join("test.pkg");
    let pkg_path = if candidate.exists() { candidate } else { Path::new("assets/dest/out.pkg").to_path_buf() };
    if !pkg_path.exists() {
        eprintln!("Package not found at {:?}", pkg_path);
        return;
    }

    let pkg = match AssetPkg::open(&pkg_path) {
        Ok(p) => p,
        Err(e) => { eprintln!("Failed to open pkg: {:?}", e); return; }
    };

    // List up to first 10 .sfx assets
    let mut sfx_names: Vec<String> = pkg.list_names().into_iter().filter(|n| n.ends_with(".sfx")).collect();
    if sfx_names.is_empty() {
        eprintln!("No .sfx assets in package");
        return;
    }
    sfx_names.truncate(10);
    println!("Found {} .sfx entries (showing up to 10):", sfx_names.len());
    for (i, name) in sfx_names.iter().enumerate() {
        println!("{}: {}", i, name);
    }

    // Create real backend and Renderer
    let mut backend = match create_audio_backend() { Ok(b) => b, Err(e) => { eprintln!("create backend failed: {:?}", e); return; } };
    let backend_sr = backend.sample_rate();
    let backend_buf = backend.buffer_size();
    let backend_channels = backend.channels() as usize;
    let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };

    println!("Using backend: sample_rate={}, channels={}, frames_per_buffer={}", backend_sr, backend_channels, frames_per_buffer);
    let renderer = Arc::new(Mutex::new(Renderer::new(backend_sr as i32, backend_channels, frames_per_buffer)));

    // create slots for each sfx we will play
    let mut slots = Vec::new();
    for _ in 0..sfx_names.len() {
        let slot = { let mut r = renderer.lock().unwrap(); r.alloc_slot().expect("no slot") };
        { let r = renderer.lock().unwrap(); let sender = r.command_sender(); sender.push(resonance_audio_engine::renderer::Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok(); }
        slots.push(slot);
    }

    // render callback with simple diagnostic: set `saw_audio` when non-zero samples produced
    let saw_audio = Arc::new(AtomicBool::new(false));
    let saw_audio_cb = Arc::clone(&saw_audio);
    let renderer_for_render = Arc::clone(&renderer);
    let render_cb = Arc::new(move |buf: &mut [f32], _sr: u32, frames_n: usize| {
        if let Ok(mut r) = renderer_for_render.try_lock() {
            let ok = r.process_output_interleaved(buf, frames_n);
            if ok {
                // detect non-zero content
                for &s in buf.iter() {
                    if s.abs() > 1e-6 {
                        saw_audio_cb.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }
        } else {
            for v in buf.iter_mut() { *v = 0.0; }
        }
    });

    backend.start(render_cb).expect("start backend");

    // Play each sfx sequentially for 2 seconds
    for (i, name) in sfx_names.iter().enumerate() {
        match pkg.read_sfx_blob(name) {
            Ok(blob) => {
                println!("Playing {} (sr={}, ch={})", name, blob.sample_rate, blob.channels);
                // resample if needed
                let mut samples = if blob.sample_rate != backend_sr as u32 {
                    asset_manager::sfx_loader::resample_interleaved(&blob.samples, blob.sample_rate, backend_sr as u32, blob.channels as usize)
                } else { blob.samples.clone() };
                // channel convert to backend channels
                let src_ch = blob.channels as usize;
                if src_ch != backend_channels {
                    if src_ch == 1 && backend_channels == 2 {
                        let frames = samples.len();
                        let mut out = Vec::with_capacity(frames*2);
                        for s in samples.iter() { out.push(*s); out.push(*s); }
                        samples = out;
                    } else if src_ch == 2 && backend_channels == 1 {
                        let mut out = Vec::with_capacity(samples.len()/2);
                        for chunk in samples.chunks(2) {
                            match chunk { [a,b] => out.push((a+b)*0.5), [a] => out.push(*a), _ => out.push(0.0) }
                        }
                        samples = out;
                    } else {
                        // fallback: pad/truncate channels per frame
                        let frames = samples.len()/src_ch.max(1);
                        let mut out = Vec::with_capacity(frames*backend_channels);
                        for f in 0..frames {
                            for ch in 0..backend_channels {
                                out.push(*samples.get(f*src_ch + ch).unwrap_or(&0.0));
                            }
                        }
                        samples = out;
                    }
                }

                let _frames_for_buffer = if backend_channels > 0 { samples.len()/backend_channels } else { 0 };
                // send PlaySfx repeatedly for a few seconds so playback is audible
                let slot = slots[i];
                let sender = { let r = renderer.lock().unwrap(); r.command_sender() };
                let meta = asset_manager::sfx_loader::SfxMetadata { channels: backend_channels as u16, sample_rate: backend_sr as u32, loop_points: None };
                let sfx_arc = std::sync::Arc::new(samples.clone());
                let play_duration = Duration::from_secs(6);
                let start = std::time::Instant::now();
                while start.elapsed() < play_duration {
                    let sfx_buffer = resonance_audio_engine::renderer::SfxBuffer { samples: sfx_arc.clone(), meta: meta.clone() };
                    sender.push(resonance_audio_engine::renderer::Command::PlaySfx { slot, buffer: sfx_buffer, gain: 1.0, pos: None }).ok();
                    // inspect renderer voice state
                    if let Ok(r) = renderer.try_lock() {
                        // count active voices
                        let active_count = r.voices.iter().filter(|v| v.active.load(std::sync::atomic::Ordering::Acquire)).count();
                        println!("Renderer active voices: {}", active_count);
                        // if this slot has sfx, print its length
                        if let Some(v) = r.voices.get(slot) {
                            if let Some(ref s) = v.sfx {
                                println!("Slot {} sfx len: {}", slot, s.len());
                            }
                        }
                    }
                    // wait a bit between pushes so playback overlaps
                    sleep(Duration::from_millis(300));
                }
            }
            Err(e) => { eprintln!("Failed to read {}: {:?}", name, e); }
        }
    }

    backend.stop().expect("stop");

    // Report diagnostic
    if saw_audio.load(Ordering::Relaxed) {
        println!("Diagnostic: render callback produced non-zero audio samples");
    } else {
        println!("Diagnostic: render callback did NOT observe non-zero audio samples");
    }
}
