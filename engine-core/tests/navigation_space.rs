use bevy_ecs::prelude::*;
use engine_core::engine::Engine;

#[derive(Resource, Default, Debug)]
struct TestEventSink {
    entered: Vec<(Entity, Entity)>,
    exited: Vec<(Entity, Entity)>,
}

fn drain_space_events(
    mut sink: ResMut<TestEventSink>,
    mut enter: ResMut<Events<engine_core::events::EnterSpaceEvent>>,
    mut exit: ResMut<Events<engine_core::events::ExitSpaceEvent>>,
) {
    // Drain all current-frame events into sink for later assertions
    let mut v = Vec::new();
    enter.drain().for_each(|e| v.push(e));
    for e in v {
        sink.entered.push((e.entity, e.space));
    }

    let mut v2 = Vec::new();
    exit.drain().for_each(|e| v2.push(e));
    for e in v2 {
        sink.exited.push((e.entity, e.space));
    }
}

#[test]
fn navigation_causes_enter_and_exit_space_events() {
    let mut eng = Engine::new();

    // Register our test sink and a system to drain events into it each update
    eng.world_mut().insert_resource(TestEventSink::default());
    eng.app_mut()
        .add_systems(bevy_app::Update, drain_space_events);

    // Create one room at x in [5, 10]
    let room = eng.create_room(
        "room-a",
        glam::vec3(5.0, -1.0, -1.0),
        glam::vec3(10.0, 1.0, 1.0),
    );

    // Create a mover entity at origin
    let mover = eng.create_entity();
    eng.ensure_navigation(mover, 100.0);

    // Move to inside the room
    eng.navigate_to(mover, glam::vec3(7.0, 0.0, 0.0), 10.0);
    // Step a few frames until target is reached
    for _ in 0..20 {
        eng.update(0.1);
    }

    // Now move out of the room back to origin
    eng.navigate_to(mover, glam::vec3(0.0, 0.0, 0.0), 10.0);
    for _ in 0..20 {
        eng.update(0.1);
    }

    // Assert we saw at least one Enter into room and one Exit from it
    let sink = eng.world().resource::<TestEventSink>();
    assert!(
        sink.entered.iter().any(|&(_e, s)| s == room),
        "expected an EnterSpaceEvent for the room"
    );
    assert!(
        sink.exited.iter().any(|&(_e, s)| s == room),
        "expected an ExitSpaceEvent for the room"
    );
}
