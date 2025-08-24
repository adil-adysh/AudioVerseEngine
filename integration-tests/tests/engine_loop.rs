use bevy_ecs::prelude::*;
use engine_core::Engine;
use engine_core::audio as ev;

/// Contract: Engine::new initializes all event resources; bootstrap wires default systems;
/// update(dt) should tick fixed schedule 0..=max_substeps times and variable once.
#[test]
fn engine_bootstrap_and_update_smoke() {
    let mut engine = Engine::new();
    engine.bootstrap();

    // create an entity and mark as listener to exercise audio_listener_system
    let listener = engine.create_entity();
    engine.set_listener(listener);
    engine.set_position(listener, glam::vec3(0.0, 0.0, 0.0));

    // step a few frames; should not panic
    for _ in 0..5 {
        engine.update(1.0 / 60.0);
    }
}

/// Fixed-step accumulator: when given a big delta, fixed steps should clamp to max_substeps
#[test]
fn fixed_step_clamps_to_max_substeps() {
    let mut engine = Engine::new();
    engine.bootstrap();

    // instrument a counter via a simple fixed system added by the test
    #[derive(Resource, Default)]
    struct Counter(u32);

    fn count_system(mut res: ResMut<Counter>) {
        res.0 += 1;
    }

    engine.world.insert_resource(Counter::default());
    engine.fixed_schedule.add_systems(count_system);

    // configure fixed dt and small max_substeps
    engine.set_fixed_dt(1.0 / 60.0, Some(3));

    // supply a huge delta that would be > 10 steps; only 3 should run
    engine.update(0.2); // ~12 frames at 60Hz

    let c = engine.world.resource::<Counter>().0;
    assert_eq!(c, 3, "fixed steps should clamp to max_substeps");
}

/// Verify PositionChangedEvent is emitted by transform system in fixed schedule
#[test]
fn emits_position_changed_on_move() {
    let mut engine = Engine::new();
    engine.bootstrap();

    let e = engine.create_entity();
    engine.set_position(e, glam::vec3(0.0, 0.0, 0.0));

    // First update will insert WorldTransform and emit a PositionChanged
    engine.update(0.016);

    // Move entity and tick again
    engine.set_position(e, glam::vec3(1.0, 0.0, 0.0));
    engine.update(0.016);

    // Read events directly
    let mut reader = bevy_ecs::event::ManualEventReader::<engine_core::audio::PositionChangedEvent>::default();
    let events = engine.world.resource::<bevy_ecs::event::Events<engine_core::audio::PositionChangedEvent>>();
    let count = reader.iter(&events).count();
    assert!(count >= 1, "expected at least one PositionChangedEvent after move");
}

/// Ensure all key event resources are present and get updated without panic
#[test]
fn event_resources_exist_and_update() {
    let mut engine = Engine::new();
    engine.bootstrap();

    // assert presence
    assert!(engine.world.contains_resource::<bevy_ecs::event::Events<ev::PlaySoundEvent>>());
    assert!(engine.world.contains_resource::<bevy_ecs::event::Events<ev::StopSoundEvent>>());
    assert!(engine.world.contains_resource::<bevy_ecs::event::Events<ev::EnterSpaceEvent>>());
    assert!(engine.world.contains_resource::<bevy_ecs::event::Events<ev::ExitSpaceEvent>>());

    // call update_event_resources at end of a frame via engine.update()
    engine.update(0.0);
}
