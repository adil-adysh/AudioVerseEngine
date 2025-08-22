use std::sync::Arc;
use std::thread;
use ringbuf::HeapRb;
use ringbuf::traits::Split;
use ringbuf::traits::Producer;
use crate::renderer::{Renderer, Command, SfxBuffer};
use asset_manager::SfxMetadata;

/// Tiny example that demonstrates game-thread usage.
pub fn demo_usage() {
    // create renderer
    let mut r = Renderer::new(48000, 2, 512);

    // allocate a slot
    let slot = r.alloc_slot().expect("no free slot");

    // create a source for this slot
    let sender = r.command_sender();
    let _ = sender.push(Command::CreateSource { slot, mode: resonance_cxx::RenderingMode::kStereoPanning });

    // load SFX via asset-manager (path must be registered in real usage)
    let meta = SfxMetadata { channels: 2, sample_rate: 48000, loop_points: None };
    let samples = Arc::new(vec![0.0f32; 1024]);
    let sfx = SfxBuffer { samples: samples.clone(), meta };

    // play SFX
    let _ = sender.push(Command::PlaySfx { slot, buffer: sfx, gain: 1.0, pos: None });

    // start a stream: create HeapRb and spawn a decoder thread that writes into producer
    let rb = HeapRb::<f32>::new(32 * 1024);
    let (mut prod, cons) = rb.split();

    // send consumer to renderer
    let _ = sender.push(Command::StartStream { slot, ring: cons, channels: 2 });

    // decoder thread: push generated samples into producer
    thread::spawn(move || {
        let buffer = vec![0.0f32; 256];
        loop {
            // produce silence or decoded samples
            let _ = prod.push_slice(&buffer);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });
}
