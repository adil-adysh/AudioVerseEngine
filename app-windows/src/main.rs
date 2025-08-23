use bevy_ecs::prelude::*;
use std::sync::Arc;

use audio_backend::{create_audio_backend, AudioBackend};
use audio_system::{render_fn_for_system, AudioSystem};
use engine_core::engine::Engine;

fn main() {
    // Create the engine and bootstrap default systems
    let mut engine = Engine::new();
    engine.bootstrap();

    // Initialize audio-system via bridge and start backend
    engine_audio::setup_audio_system(&mut engine.world, 48_000, 2, 128);
    let sys = engine.world.resource::<engine_audio::AudioSystemRes>().0.clone();
    let mut backend = create_audio_backend().expect("audio backend");
    backend
        .start(render_fn_for_system(sys.clone()))
        .expect("start backend");

    // Create a listener and a source entity
    let listener = engine.create_entity();
    engine.set_listener(listener);
    engine.set_position(listener, glam::Vec3::new(0.0, 1.6, 0.0));

    let source = engine.create_entity();
    engine.add_sound(source, "sine:440");
    engine.set_position(source, glam::Vec3::new(1.0, 1.6, 0.0));
    engine.play(source);

    // Simple run loop for a short demo (~2 seconds at 60 FPS)
    for _ in 0..120 {
        engine.update(1.0 / 60.0);
    }
}
