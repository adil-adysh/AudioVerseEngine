use bevy_ecs::prelude::*;
use engine_core::engine::Engine;
use engine_core::components::{Shape3D, Aabb, SpaceKind, MediumType};

#[derive(Resource, Default, Debug)]
struct TestEventSink {
    exited: Vec<(Entity, Entity)>,
}

fn drain_exit_events(
    mut sink: ResMut<TestEventSink>,
    mut exit: ResMut<Events<engine_core::audio::ExitSpaceEvent>>,
) {
    let mut v = Vec::new();
    exit.drain().for_each(|e| v.push(e));
    for e in v { sink.exited.push((e.entity, e.space)); }
}

#[test]
fn can_exit_no_ceiling_when_can_climb() {
    let mut eng = Engine::new();
    eng.world_mut().insert_resource(TestEventSink::default());
    eng.app_mut().add_systems(bevy_app::Update, drain_exit_events);

    // Space with no ceiling
    let space = eng.create_space_with_shape(
        "pit",
        SpaceKind::Room,
        Shape3D::Aabb(Aabb { min: glam::vec3(-2.0, -2.0, -2.0), max: glam::vec3(2.0, 2.0, 2.0) }),
        false,
        MediumType::Air,
    );

    let mover = eng.create_entity();
    eng.ensure_navigation(mover, 10.0);
    eng.set_position(mover, glam::vec3(0.0, 0.0, 0.0));

    // Without climb ability, try to leave: should be blocked (no Exit)
    eng.navigate_to(mover, glam::vec3(3.0, 0.0, 0.0), 10.0);
    for _ in 0..40 { eng.update(0.05); }
    let sink = eng.world().resource::<TestEventSink>();
    assert!(!sink.exited.iter().any(|&(_e, s)| s == space), "exit should be blocked without CanClimb");

    // Grant climb ability and try again: should exit now
    let _ = sink;
    eng.grant_climb(mover);
    eng.navigate_to(mover, glam::vec3(3.0, 0.0, 0.0), 10.0);
    for _ in 0..40 { eng.update(0.05); }
    let sink = eng.world().resource::<TestEventSink>();
    assert!(sink.exited.iter().any(|&(_e, s)| s == space), "expected ExitSpaceEvent after CanClimb granted");
}
