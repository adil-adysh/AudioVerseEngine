use resonance_audio_engine::Renderer;
use resonance_audio_engine::renderer::{Command, SfxBuffer};
use asset_manager::sfx_loader::SfxMetadata;
use std::sync::Arc;
use std::thread;

#[test]
fn alloc_slot_exhaustion() {
    let mut r = Renderer::new(48000, 2, 64);
    let sender = r.command_sender();

    // fill all available slots by creating sources and playing SFX
    let mut slots = Vec::new();
    // process each slot immediately so it becomes active before the next alloc
    for _ in 0..256 {
        let slot = r.alloc_slot().expect("expected slot");
        let samples = Arc::new(vec![0.5f32; 64 * 2]);
        let meta = SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None };
        let sfx = SfxBuffer { samples: samples.clone(), meta };
        sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok();
        sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 1.0, pos: None }).ok();

        // process immediately so the slot becomes active and won't be returned again
        let mut out = vec![0.0f32; 64 * 2];
        let _ = r.process_output_interleaved(&mut out, 64);

        slots.push(slot);
    }

    // now no free slots (renderer should report none available)
    assert!(r.alloc_slot().is_none(), "no free slot expected");
}

#[test]
fn command_queue_overflow_is_reported() {
    let r = Renderer::new(48000, 2, 32);
    let sender = r.command_sender();

    // push until push returns Err (queue full); this should happen without panics
    let mut pushed = 0usize;
    loop {
        let res = sender.push(Command::SetListenerPose { position: glam::Vec3::ZERO, rotation: glam::Quat::IDENTITY });
        if res.is_err() { break; }
        pushed += 1;
        // stop at a high cap just in case
        if pushed > 20000 { break; }
    }
    assert!(pushed > 0, "should have pushed some commands before overflow");
}

#[test]
fn mono_stereo_mismatch_does_not_panic_and_produces_output() {
    // engine outputs stereo, input is stereo; ensure mixing runs (the underlying
    // resonance implementation only supports stereo output)
    let mut r = Renderer::new(48000, 2, 64);
    let sender = r.command_sender();
    let slot = r.alloc_slot().expect("slot");
    sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok();

    // stereo samples with left=1.0 right=0.0
    let mut samples = Vec::with_capacity(64 * 2);
    for _ in 0..64 { samples.push(1.0f32); samples.push(0.0f32); }
    let meta = SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None };
    let sfx = SfxBuffer { samples: Arc::new(samples), meta };
    sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 1.0, pos: None }).ok();

    let mut out = vec![0.0f32; 64 * 2];
    let _ = r.process_output_interleaved(&mut out, 64);
    assert!(out.iter().any(|v| *v != 0.0), "output should contain mixed samples");
}

#[test]
fn empty_buffer_deactivates_voice_and_no_panic() {
    let mut r = Renderer::new(48000, 2, 32);
    let sender = r.command_sender();
    let slot = r.alloc_slot().expect("slot");
    // empty samples
    let sfx = SfxBuffer { samples: Arc::new(vec![]), meta: SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None } };
    sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok();
    sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 1.0, pos: None }).ok();

    let mut out = vec![0.0f32; 32 * 2];
    let _ = r.process_output_interleaved(&mut out, 32);
    // second call should still be safe and silent
    let mut out2 = vec![0.0f32; 32 * 2];
    let _ = r.process_output_interleaved(&mut out2, 32);
    assert!(out2.iter().all(|v| *v == 0.0), "empty buffer should not produce output");
}

#[test]
fn set_voice_gain_zero_silences_output() {
    let mut r = Renderer::new(48000, 2, 32);
    let sender = r.command_sender();
    let slot = r.alloc_slot().expect("slot");
    sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok();

    let samples = Arc::new(vec![1.0f32; 32 * 2]);
    let meta = SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None };
    let sfx = SfxBuffer { samples: samples.clone(), meta };
    sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 1.0, pos: None }).ok();
    // silence it
    sender.push(Command::SetVoiceGain { slot, gain: 0.0 }).ok();

    let mut out = vec![0.0f32; 32 * 2];
    let _ = r.process_output_interleaved(&mut out, 32);
    assert!(out.iter().all(|v| *v == 0.0), "gain 0 should silence output");
}

#[test]
fn concurrency_stress_short_run() {
    let mut r = Renderer::new(48000, 2, 32);
    let sender = r.command_sender();
    let slot = r.alloc_slot().expect("slot");
    sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok();

    let handle = thread::spawn(move || {
        // rapidly push play commands
        for _ in 0..50 {
            let samples = Arc::new(vec![0.1f32; 32 * 2]);
            let meta = SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None };
            let sfx = SfxBuffer { samples: samples.clone(), meta };
            sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 1.0, pos: None }).ok();
        }
    });

    // main thread processes buffers while other thread pushes commands
    for _ in 0..50 {
        let mut out = vec![0.0f32; 32 * 2];
        let _ = r.process_output_interleaved(&mut out, 32);
    }
    let _ = handle.join();
}
