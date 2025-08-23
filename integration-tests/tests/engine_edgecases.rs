use engine_core::{Engine, components::*, events as ev};
use glam::{vec3, Vec3};

fn step(engine: &mut Engine, seconds: f32, frames: usize) { for _ in 0..frames { engine.update(seconds / frames as f32); } }

#[test]
fn zero_and_negative_dt_do_not_move_or_panic() {
    let mut engine = Engine::new();
    engine.bootstrap();
    let e = engine.create_entity();
    engine.ensure_navigation(e, 10.0);
    engine.set_position(e, Vec3::ZERO);
    engine.navigate_to(e, vec3(10.0, 0.0, 0.0), 10.0);

    // zero dt
    engine.update(0.0);
    // negative dt is clamped to 0
    engine.update(-1.0);
    // position should remain at origin
    // Read transform component directly
    let pos = engine.world.get::<TransformComponent>(e).unwrap().position;
    assert_eq!(pos, Vec3::ZERO);
}

#[test]
fn huge_dt_clamps_fixed_and_reaches_target() {
    let mut engine = Engine::new();
    engine.bootstrap();
    let e = engine.create_entity();
    engine.ensure_navigation(e, 1000.0);
    engine.set_position(e, Vec3::ZERO);
    engine.navigate_to(e, vec3(1.0, 0.0, 0.0), 1000.0);

    // a big dt triggers fixed-step clamping but variable step should carry to target
    engine.set_fixed_dt(1.0 / 120.0, Some(2));
    engine.update(1.0);
    // should be at or past target and nav cleared
    let t = engine.world.get::<TransformComponent>(e).unwrap();
    assert!((t.position - vec3(1.0,0.0,0.0)).length() < 1e-3);
}

#[test]
fn portal_policy_enforced_for_tags_and_abilities() {
    let mut engine = Engine::new();
    engine.bootstrap();

    // build two spaces with a portal requiring tag "player" and ability "dive"
    let a = engine.create_space_with_shape("A", SpaceKind::Room, Shape3D::Aabb(Aabb{min:vec3(0.0,0.0,0.0), max:vec3(5.0,3.0,5.0)}), true, MediumType::Air);
    let b = engine.create_space_with_shape("B", SpaceKind::Ocean, Shape3D::Aabb(Aabb{min:vec3(6.0,0.0,0.0), max:vec3(12.0,3.0,5.0)}), false, MediumType::Water);
    let door = engine.add_portal_with_policy(a, b, Shape3D::Aabb(Aabb{min:vec3(4.5,0.0,2.0), max:vec3(6.5,2.0,3.0)}), true, true, &["player"], &["dive"], 1.0);
    let _ = door;

    let actor = engine.create_entity();
    engine.ensure_navigation(actor, 5.0);
    engine.set_position(actor, vec3(1.0, 1.0, 1.0));

    // without tags/abilities, should not exit Room into Ocean when moving toward it
    engine.navigate_to(actor, vec3(8.0, 1.0, 2.5), 5.0);
    step(&mut engine, 2.0, 120);

    // actor should still be inside A only
    let inside = engine.world.get::<InsideSpaces>(actor).cloned().unwrap_or_default();
    assert!(inside.spaces.contains(&a));
    assert!(!inside.spaces.contains(&b));

    // now grant tag and ability and try again
    engine.set_traversal_tags(actor, ["player"]);
    {
    let mut abil = engine.world.resource_mut::<AbilityRegistry>();
    let mask = abil.mask_for(["dive"]);
        engine.world.entity_mut(actor).insert(Abilities{mask});
    }
    engine.set_position(actor, vec3(1.0,1.0,1.0));
    engine.navigate_to(actor, vec3(8.0, 1.0, 2.5), 5.0);
    step(&mut engine, 2.0, 120);

    let inside = engine.world.get::<InsideSpaces>(actor).cloned().unwrap_or_default();
    assert!(inside.spaces.contains(&b), "actor should pass through when requirements met");
}

#[test]
fn listener_event_fires_with_transform() {
    let mut engine = Engine::new(); engine.bootstrap();
    let l = engine.create_entity();
    engine.set_listener(l);
    engine.set_position(l, vec3(0.0,0.0,0.0));
    // first update computes world transform and should emit ListenerTransformEvent
    engine.update(0.016);
    let mut reader = bevy_ecs::event::ManualEventReader::<ev::ListenerTransformEvent>::default();
    let evs = engine.world.resource::<bevy_ecs::event::Events<ev::ListenerTransformEvent>>();
    assert!(reader.iter(evs).next().is_some());
}
