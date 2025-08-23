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
    let mut v = Vec::new();
    enter.drain().for_each(|e| v.push(e));
    for e in v { sink.entered.push((e.entity, e.space)); }

    let mut v2 = Vec::new();
    exit.drain().for_each(|e| v2.push(e));
    for e in v2 { sink.exited.push((e.entity, e.space)); }
}

#[test]
fn portal_allows_tagged_entity() {
    let mut eng = Engine::new();
    eng.bootstrap();
    eng.world.insert_resource(TestEventSink::default());
    eng.variable_schedule.add_systems(drain_space_events);

    // Two rooms side by side; door portal in between at x ~ 5.0
    let room_a = eng.create_room("A", glam::vec3(0.0, -1.0, -1.0), glam::vec3(4.0, 1.0, 1.0));
    let room_b = eng.create_room("B", glam::vec3(6.0, -1.0, -1.0), glam::vec3(10.0, 1.0, 1.0));

    // Portal centered at x=5.0, small opening
    let door = engine_core::components::Shape3D::Aabb(engine_core::components::Aabb { min: glam::vec3(4.8, -0.5, -0.5), max: glam::vec3(5.2, 0.5, 0.5) });
    let _portal = eng.add_portal(room_a, room_b, door, true, true, Some(vec!["player".to_string()]));

    // Mover starts left of both rooms and will traverse through the door line
    let mover = eng.create_entity();
    eng.ensure_navigation(mover, 20.0);
    eng.set_position(mover, glam::vec3(-2.0, 0.0, 0.0));

    // Give it the tag required by the portal
    eng.set_traversal_tags(mover, ["player".to_string()]);

    // Navigate to inside room B, crossing the portal
    eng.navigate_to(mover, glam::vec3(8.0, 0.0, 0.0), 20.0);
    for _ in 0..80 { eng.update(0.05); }

    // We should have seen an Exit from room A and Enter into room B at some point
    let sink = eng.world.resource::<TestEventSink>();
    assert!(sink.exited.iter().any(|&(_e, s)| s == room_a), "expected an ExitSpaceEvent from room A");
    assert!(sink.entered.iter().any(|&(_e, s)| s == room_b), "expected an EnterSpaceEvent into room B");
}

#[test]
fn portal_blocks_untagged_entity() {
    let mut eng = Engine::new();
    eng.bootstrap();
    eng.world.insert_resource(TestEventSink::default());
    eng.variable_schedule.add_systems(drain_space_events);

    let room_a = eng.create_room("A", glam::vec3(0.0, -1.0, -1.0), glam::vec3(4.0, 1.0, 1.0));
    let room_b = eng.create_room("B", glam::vec3(6.0, -1.0, -1.0), glam::vec3(10.0, 1.0, 1.0));
    let door = engine_core::components::Shape3D::Aabb(engine_core::components::Aabb { min: glam::vec3(4.8, -0.5, -0.5), max: glam::vec3(5.2, 0.5, 0.5) });
    let _portal = eng.add_portal(room_a, room_b, door, true, true, Some(vec!["player".to_string()]));

    let mover = eng.create_entity();
    eng.ensure_navigation(mover, 20.0);
    eng.set_position(mover, glam::vec3(-2.0, 0.0, 0.0));

    // No traversal tags

    eng.navigate_to(mover, glam::vec3(8.0, 0.0, 0.0), 20.0);
    for _ in 0..80 { eng.update(0.05); }

    // Because portal requires tag, exit from room A should be blocked (no ExitSpaceEvent for A)
    let sink = eng.world.resource::<TestEventSink>();
    assert!(!sink.exited.iter().any(|&(_e, s)| s == room_a), "room A exit should be blocked for untagged entity");
}
