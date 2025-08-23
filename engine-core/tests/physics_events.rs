use bevy_ecs::prelude::*;

#[test]
fn physics_event_resource_registered() {
    let mut world = World::default();
    // call existing helper to register audio/play/listener events
    engine_core::events::init_event_resources(&mut world);
    // register physics events via the new helper
    engine_core::engine::Engine::init_physics_events(&mut world);

    // ensure the resource exists
    let _ = world.resource::<Events<engine_core::events::PhysicsCollisionEvent>>();
}
