use asset_manager::asset_pkg::AssetPkg;
use audio_backend::create_audio_backend;
use audio_system::AudioWorld;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // Find package
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("tests").join("test.pkg");
    let pkg_path = if candidate.exists() {
        candidate
    } else {
        Path::new("assets/dest/out.pkg").to_path_buf()
    };
    if !pkg_path.exists() {
        eprintln!("Package not found at {:?}", pkg_path);
        return;
    }

    let pkg = match AssetPkg::open(&pkg_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to open pkg: {:?}", e);
            return;
        }
    };

    // List up to first 10 .sfx assets
    let mut sfx_names: Vec<String> = pkg
        .list_names()
        .into_iter()
        .filter(|n| n.ends_with(".sfx"))
        .collect();
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
    let mut backend = match create_audio_backend() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("create backend failed: {:?}", e);
            return;
        }
    };
    let backend_sr = backend.sample_rate();
    let backend_buf = backend.buffer_size();
    let backend_channels = backend.channels() as usize;
    let frames_per_buffer = if backend_buf == 0 { 256 } else { backend_buf };

    println!(
        "Using backend: sample_rate={}, channels={}, frames_per_buffer={}",
        backend_sr, backend_channels, frames_per_buffer
    );

    // Create native Api + AudioWorld and wrap in Arc<Mutex<..>> for sharing with render thread
    let ra_api =
        match resonance_cxx::Api::new(backend_channels, frames_per_buffer, backend_sr as i32) {
            Some(a) => a,
            None => {
                eprintln!("create resonance Api failed");
                return;
            }
        };
    let audio_world = Arc::new(Mutex::new(AudioWorld::new(ra_api)));

    // create entity ids and add audio sources for each sfx
    let mut entities: Vec<u32> = Vec::new();
    for i in 0..sfx_names.len() {
        let entity = (i as u32) + 1;
        {
            let mut aw = audio_world.lock().unwrap();
            aw.add_audio_source(entity, resonance_cxx::RenderingMode::kStereoPanning);
        }
        entities.push(entity);
    }
    // prepare world for RT use
    {
        let mut aw = audio_world.lock().unwrap();
        aw.prepare_for_rt();
    }

    // render callback with simple diagnostic: set `saw_audio` when non-zero samples produced
    let saw_audio = Arc::new(AtomicBool::new(false));
    let saw_audio_cb = Arc::clone(&saw_audio);
    let audio_world_for_render = Arc::clone(&audio_world);
    let render_cb = Arc::new(move |buf: &mut [f32], sample_rate: u32, frames_n: usize| {
        let _ = sample_rate;
        if let Ok(mut aw) = audio_world_for_render.try_lock() {
            // Let the native API fill the buffer using previously provided source buffers
            let ok = aw.api_fill_interleaved_f32(backend_channels, frames_n, buf);
            if ok {
                for &s in buf.iter() {
                    if s.abs() > 1e-6 {
                        saw_audio_cb.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }
        } else {
            for v in buf.iter_mut() {
                *v = 0.0;
            }
        }
    });

    backend.start(render_cb).expect("start backend");

    // Play each sfx sequentially for 2 seconds
    for (i, name) in sfx_names.iter().enumerate() {
        match pkg.read_sfx_blob(name) {
            Ok(blob) => {
                println!(
                    "Playing {} (sr={}, ch={})",
                    name, blob.sample_rate, blob.channels
                );
                // resample if needed
                let mut samples = if blob.sample_rate != backend_sr as u32 {
                    asset_manager::sfx_loader::resample_interleaved(
                        &blob.samples,
                        blob.sample_rate,
                        backend_sr as u32,
                        blob.channels as usize,
                    )
                } else {
                    blob.samples.clone()
                };
                // channel convert to backend channels
                let src_ch = blob.channels as usize;
                if src_ch != backend_channels {
                    if src_ch == 1 && backend_channels == 2 {
                        let frames = samples.len();
                        let mut out = Vec::with_capacity(frames * 2);
                        for s in samples.iter() {
                            out.push(*s);
                            out.push(*s);
                        }
                        samples = out;
                    } else if src_ch == 2 && backend_channels == 1 {
                        let mut out = Vec::with_capacity(samples.len() / 2);
                        for chunk in samples.chunks(2) {
                            match chunk {
                                [a, b] => out.push((a + b) * 0.5),
                                [a] => out.push(*a),
                                _ => out.push(0.0),
                            }
                        }
                        samples = out;
                    } else {
                        // fallback: pad/truncate channels per frame
                        let frames = samples.len() / src_ch.max(1);
                        let mut out = Vec::with_capacity(frames * backend_channels);
                        for f in 0..frames {
                            for ch in 0..backend_channels {
                                out.push(*samples.get(f * src_ch + ch).unwrap_or(&0.0));
                            }
                        }
                        samples = out;
                    }
                }

                let _frames_for_buffer = if backend_channels > 0 {
                    samples.len() / backend_channels
                } else {
                    0
                };
                // send PlaySfx repeatedly for a few seconds so playback is audible
                let entity = entities[i];
                let meta = asset_manager::sfx_loader::SfxMetadata {
                    channels: backend_channels as u16,
                    sample_rate: backend_sr as u32,
                    loop_points: None,
                };
                let play_duration = Duration::from_secs(6);
                let mut playhead_frames = 0usize;
                let total_frames = samples.len() / backend_channels.max(1);
                let start = std::time::Instant::now();
                while start.elapsed() < play_duration {
                    // compute chunk for this iteration
                    let frames_to_send =
                        (frames_per_buffer).min(total_frames.saturating_sub(playhead_frames));
                    let start_sample = playhead_frames * backend_channels;
                    let end_sample = start_sample + frames_to_send * backend_channels;
                    if start_sample < end_sample && end_sample <= samples.len() {
                        let chunk = &samples[start_sample..end_sample];
                        {
                            let mut aw = audio_world.lock().unwrap();
                            aw.feed_audio(entity, chunk, backend_channels, frames_to_send);
                        }
                        playhead_frames = playhead_frames.saturating_add(frames_to_send);
                    } else {
                        // no more frames; break early
                        break;
                    }
                    sleep(Duration::from_millis(300));
                }
            }
            Err(e) => {
                eprintln!("Failed to read {}: {:?}", name, e);
            }
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
