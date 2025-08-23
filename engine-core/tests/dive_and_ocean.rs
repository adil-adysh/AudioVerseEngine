use bevy_ecs::prelude::*;
use engine_core::engine::Engine;

#[derive(Resource, Default, Debug)]
struct TestEventSink {
    entered: Vec<(Entity, Entity)>,
}

fn drain_enter_events(
    mut sink: ResMut<TestEventSink>,
    mut enter: ResMut<Events<engine_core::events::EnterSpaceEvent>>,
) {
    let mut v = Vec::new();
    enter.drain().for_each(|e| v.push(e));
    for e in v {
        sink.entered.push((e.entity, e.space));
    }
}

#[test]
fn can_enter_ocean_only_with_dive() {
    let mut eng = Engine::new();
    eng.world_mut().insert_resource(TestEventSink::default());
    eng.app_mut().add_systems(bevy_app::Update, drain_enter_events);

    // Create ocean at x in [5,10]
    let ocean = eng.create_ocean(
        "ocean",
        glam::vec3(5.0, -5.0, -5.0),
        glam::vec3(10.0, 5.0, 5.0),
    );

    let mover = eng.create_entity();
    eng.ensure_navigation(mover, 20.0);
    eng.set_position(mover, glam::vec3(0.0, 0.0, 0.0));

    // Try to navigate into water without dive: should not Enter
    eng.navigate_to(mover, glam::vec3(7.0, 0.0, 0.0), 20.0);
    for _ in 0..80 {
        eng.update(0.05);
    }
    let sink = eng.world().resource::<TestEventSink>();
    assert!(
        !sink.entered.iter().any(|&(_e, s)| s == ocean),
        "should not enter water without CanDive"
    );

    let _ = sink;
    // Grant dive ability and try again
    eng.grant_dive(mover);
    eng.navigate_to(mover, glam::vec3(7.0, 0.0, 0.0), 20.0);
    for _ in 0..80 {
        eng.update(0.05);
    }
    let sink = eng.world().resource::<TestEventSink>();
    assert!(
        sink.entered.iter().any(|&(_e, s)| s == ocean),
        "expected EnterSpaceEvent after CanDive granted"
    );
}
