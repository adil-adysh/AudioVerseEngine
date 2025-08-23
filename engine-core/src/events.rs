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

/// Pause sound event payload
#[derive(Debug, Clone)]
pub struct PauseSoundEvent {
    pub entity: Entity,
}

/// Set volume event payload
#[derive(Debug, Clone)]
pub struct SetVolumeEvent {
    pub entity: Entity,
    pub volume: f32,
}

/// Listener transform update payload
#[derive(Debug, Clone)]
pub struct ListenerTransformEvent {
    pub entity: Entity,
    pub matrix: glam::Mat4,
}

// Bevy's Events require the Event trait bound. Provide empty impls to
// satisfy the trait bounds for these plain POD event types.
impl bevy_ecs::event::Event for PlaySoundEvent {}
impl bevy_ecs::event::Event for StopSoundEvent {}
impl bevy_ecs::event::Event for PauseSoundEvent {}
impl bevy_ecs::event::Event for SetVolumeEvent {}
impl bevy_ecs::event::Event for ListenerTransformEvent {}

/// Asset/resource management events
#[derive(Debug, Clone)]
pub struct LoadAssetEvent { pub asset_id: String }
impl bevy_ecs::event::Event for LoadAssetEvent {}

#[derive(Debug, Clone)]
pub struct ReleaseAssetEvent { pub asset_id: String }
impl bevy_ecs::event::Event for ReleaseAssetEvent {}

/// Streaming open/close events
#[derive(Debug, Clone)]
pub struct OpenAudioStreamEvent { pub asset_id: String }
impl bevy_ecs::event::Event for OpenAudioStreamEvent {}

#[derive(Debug, Clone)]
pub struct CloseAudioStreamEvent { pub stream_id: u64 }
impl bevy_ecs::event::Event for CloseAudioStreamEvent {}

// --- Acoustics events ---
#[derive(Debug, Clone)]
pub enum AcousticsEvent {
    MediumChanged { entity: Entity, from: crate::components::MediumType, to: crate::components::MediumType },
    RoomEntered { entity: Entity, room: Entity, material: Option<String> },
    RoomExited { entity: Entity, room: Entity },
}
impl bevy_ecs::event::Event for AcousticsEvent {}

// Helper: register Bevy Events resources at bootstrap time
pub fn init_event_resources(world: &mut World) {
    world.insert_resource(Events::<PlaySoundEvent>::default());
    world.insert_resource(Events::<StopSoundEvent>::default());
    world.insert_resource(Events::<PauseSoundEvent>::default());
    world.insert_resource(Events::<SetVolumeEvent>::default());
    world.insert_resource(Events::<ListenerTransformEvent>::default());
    world.insert_resource(Events::<LoadAssetEvent>::default());
    world.insert_resource(Events::<ReleaseAssetEvent>::default());
    world.insert_resource(Events::<OpenAudioStreamEvent>::default());
    world.insert_resource(Events::<CloseAudioStreamEvent>::default());
    world.insert_resource(Events::<AcousticsEvent>::default());
    // NavMesh cue events
    world.insert_resource(Events::<BoundaryProximityEvent>::default());
    world.insert_resource(Events::<WayfindingCueEvent>::default());
    world.insert_resource(Events::<OcclusionEstimateEvent>::default());
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

/// Update all known event resources to advance frame readers.
pub fn update_event_resources(world: &mut World) {
    world.resource_mut::<Events<PlaySoundEvent>>().update();
    world.resource_mut::<Events<StopSoundEvent>>().update();
    world.resource_mut::<Events<PauseSoundEvent>>().update();
    world.resource_mut::<Events<SetVolumeEvent>>().update();
    world.resource_mut::<Events<ListenerTransformEvent>>().update();
    world.resource_mut::<Events<LoadAssetEvent>>().update();
    world.resource_mut::<Events<ReleaseAssetEvent>>().update();
    world.resource_mut::<Events<OpenAudioStreamEvent>>().update();
    world.resource_mut::<Events<CloseAudioStreamEvent>>().update();
    world.resource_mut::<Events<AcousticsEvent>>().update();
    // NavMesh cue events
    if let Some(mut ev) = world.get_resource_mut::<Events<BoundaryProximityEvent>>() { ev.update(); }
    if let Some(mut ev) = world.get_resource_mut::<Events<WayfindingCueEvent>>() { ev.update(); }
    if let Some(mut ev) = world.get_resource_mut::<Events<OcclusionEstimateEvent>>() { ev.update(); }
    if let Some(mut ev) = world.get_resource_mut::<Events<PhysicsCollisionEvent>>() {
        ev.update();
    }
}

// --- Navigation & Space Events ---
#[derive(Debug, Clone)]
pub struct NavigateToEvent { pub entity: Entity, pub target: glam::Vec3, pub speed: f32 }
impl bevy_ecs::event::Event for NavigateToEvent {}

#[derive(Debug, Clone)]
pub struct EnterSpaceEvent { pub entity: Entity, pub space: Entity }
impl bevy_ecs::event::Event for EnterSpaceEvent {}

#[derive(Debug, Clone)]
pub struct ExitSpaceEvent { pub entity: Entity, pub space: Entity }
impl bevy_ecs::event::Event for ExitSpaceEvent {}

/// Register navigation/space events in init_event_resources.
/// Note: called from Engine::new via init_event_resources above; we extend the
/// registration by re-inserting if missing to preserve idempotency in tests.
pub fn ensure_space_nav_events(world: &mut World) {
    if world.get_resource::<Events<NavigateToEvent>>().is_none() {
        world.insert_resource(Events::<NavigateToEvent>::default());
    }
    if world.get_resource::<Events<EnterSpaceEvent>>().is_none() {
        world.insert_resource(Events::<EnterSpaceEvent>::default());
    }
    if world.get_resource::<Events<ExitSpaceEvent>>().is_none() {
        world.insert_resource(Events::<ExitSpaceEvent>::default());
    }
}

// --- NavMesh cue events ---
#[derive(Debug, Clone)]
pub struct BoundaryProximityEvent { pub entity: Entity, pub distance: f32 }
impl bevy_ecs::event::Event for BoundaryProximityEvent {}

#[derive(Debug, Clone)]
pub struct WayfindingCueEvent { pub entity: Entity, pub target: glam::Vec3, pub turn: f32 }
impl bevy_ecs::event::Event for WayfindingCueEvent {}

#[derive(Debug, Clone)]
pub struct OcclusionEstimateEvent { pub entity: Entity, pub source: Entity, pub occlusion: f32 }
impl bevy_ecs::event::Event for OcclusionEstimateEvent {}
