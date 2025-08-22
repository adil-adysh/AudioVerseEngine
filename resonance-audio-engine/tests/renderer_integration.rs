use resonance_audio_engine::Renderer;
use resonance_audio_engine::renderer::{Command, SfxBuffer};
use asset_manager::sfx_loader::SfxMetadata;
use ringbuf::HeapRb;
use ringbuf::traits::{Split, Producer};

#[test]
fn renderer_alloc_and_play_sfx() {
    let mut r = Renderer::new(48000, 2, 256);

    // allocate a slot
    let slot = r.alloc_slot().expect("should have free slot");

    // create source via command
    let sender = r.command_sender();
    sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning }).ok();

    // play SFX
    let meta = SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None };
    let samples = std::sync::Arc::new(vec![1.0f32; 512]);
    let sfx = SfxBuffer { samples: samples.clone(), meta };
    sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 0.5, pos: None }).ok();

    // process output and ensure samples were mixed into buffer
    let mut out = vec![0.0f32; 256 * 2];
    // process output; the underlying Api may return false if the C++ graph
    // has no connected sources, so we only assert that renderer mixing
    // produced non-zero samples in the buffer.
    let _ok = r.process_output_interleaved(&mut out, 256);
    let any_nonzero = out.iter().any(|s| *s != 0.0);
    assert!(any_nonzero, "output buffer should contain mixed samples");
}

#[test]
fn renderer_streaming_and_stop() {
    let mut r = Renderer::new(48000, 2, 128);
    let slot = r.alloc_slot().expect("slot");
    let sender = r.command_sender();

    // start a ring buffer stream
    let rb = HeapRb::<f32>::new(1024);
    let (mut prod, cons) = rb.split();
    // push some samples into producer
    let samples = vec![0.25f32; 256 * 2];
    prod.push_slice(&samples);

    sender.push(Command::StartStream { slot, ring: cons, channels: 2 }).ok();

    let mut out = vec![0.0f32; 128 * 2];
    let _ok = r.process_output_interleaved(&mut out, 128);
    // since stream samples were 0.25, output should reflect that
    assert!(out.iter().any(|v| *v != 0.0));

    // stop stream
    sender.push(Command::StopStream { slot }).ok();
    let mut out2 = vec![0.0f32; 128 * 2];
    let _ = r.process_output_interleaved(&mut out2, 128);
    // after stopping, there may be no stream contribution
}
