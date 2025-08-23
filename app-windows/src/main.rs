use bevy_ecs::prelude::*;
use std::sync::Arc;

// Bring in the workspace crates
use audio_system::AudioSystem;
use engine_core::events::PlaySoundEvent;

/// Resources to hold shared audio system
#[derive(Resource)]
struct SharedAudio(Arc<AudioSystem>);

fn main() {
    // Initialize the audio system
    let audio = Arc::new(AudioSystem::new(64, 48000, 128).expect("audio system"));
    audio.initialize();

    // Create a World and add resources (SharedAudio + Bevy Events)
    let mut world = World::new();
    // Register the engine event resources so Events<PlaySoundEvent> exists
    engine_core::events::init_event_resources(&mut world);
    world.insert_resource(SharedAudio(audio));

    // Build a schedule with two systems: emit (startup) and consume
    let mut schedule = Schedule::default();
    schedule.add_systems(emit_demo_play_sound_system);
    schedule.add_systems(consume_play_sound_events_system);

    // Run the schedule once for this demo
    schedule.run(&mut world);
}

/// Startup-style system (runs once) to emit a PlaySoundEvent into the engine Events
fn emit_demo_play_sound_system(mut events: ResMut<Events<PlaySoundEvent>>) {
    // Emit a play for entity handle 1
    events.send(PlaySoundEvent {
        entity: Entity::from_raw(1),
    });
}

/// System that consumes PlaySoundEvent events and calls into audio-system helper
fn consume_play_sound_events_system(
    mut reader: Local<EventReader<PlaySoundEvent>>,
    events: Res<Events<PlaySoundEvent>>,
    audio_res: Res<SharedAudio>,
) {
    for ev in reader.iter(&events) {
        audio_system::handle_play_sound_event(&audio_res.0, ev.clone());
    }
}
