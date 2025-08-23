use bevy_ecs::prelude::*;

/// Play sound event payload
#[derive(Debug, Clone)]
pub struct PlaySoundEvent {
    pub entity: Entity,
}

/// Stop sound event payload
#[derive(Debug, Clone)]
pub struct StopSoundEvent {
    pub entity: Entity,
}

/// Listener transform update payload
#[derive(Debug, Clone)]
pub struct ListenerTransformEvent {
    pub entity: Entity,
}

// Bevy's Events require the Event trait bound. Provide empty impls to
// satisfy the trait bounds for these plain POD event types.
impl bevy_ecs::event::Event for PlaySoundEvent {}
impl bevy_ecs::event::Event for StopSoundEvent {}
impl bevy_ecs::event::Event for ListenerTransformEvent {}

// Helper: register Bevy Events resources at bootstrap time
pub fn init_event_resources(world: &mut World) {
    world.insert_resource(Events::<PlaySoundEvent>::default());
    world.insert_resource(Events::<StopSoundEvent>::default());
    world.insert_resource(Events::<ListenerTransformEvent>::default());
}

/// Physics collision event payload
#[derive(Debug, Clone)]
pub struct PhysicsCollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub contact_point: [f32; 3],
    pub relative_velocity: [f32; 3],
    pub impulse: f32,
    pub materials: (String, String),
}

impl bevy_ecs::event::Event for PhysicsCollisionEvent {}

// Register physics event resource as well
impl super::engine::Engine {
    pub fn init_physics_events(world: &mut bevy_ecs::prelude::World) {
        world.insert_resource(bevy_ecs::prelude::Events::<PhysicsCollisionEvent>::default());
    }
}
