use bevy_ecs::prelude::*;

use crate::components::*;
use crate::events::*;
use crate::systems::*;
use crate::transform::*;

pub struct Engine {
    pub world: World,
    pub fixed_schedule: Schedule,
    pub variable_schedule: Schedule,
}

impl Engine {
    pub fn new() -> Self {
        let mut world = World::new();
    init_event_resources(&mut world);

    let fixed_schedule = Schedule::default();
    let variable_schedule = Schedule::default();

        // Systems will be added by consumers / bootstrap code later.

        Self {
            world,
            fixed_schedule,
            variable_schedule,
        }
    }

    /// Called once to register engine-provided systems and do one-time setup.
    pub fn bootstrap(&mut self) {
        // Register physics resources and systems. Applications can still add
        // or replace systems as needed.
        use crate::physics::{PhysicsResources, physics_spawn_system, physics_step_system};

        // insert physics resources if missing
        if self.world.get_resource::<PhysicsResources>().is_none() {
            self.world.insert_resource(PhysicsResources::default());
        }

    // Ensure physics collision event resource exists for systems that emit it
    Self::init_physics_events(&mut self.world);

    // add transforms and physics systems to fixed schedule
    self.fixed_schedule.add_systems((physics_transform_system, physics_spawn_system, physics_step_system));

    // variable schedule: navigation setup, movement step, listener, render transforms, audio, space membership
    self.variable_schedule.add_systems((navigation_system, navigation_step_system, audio_listener_system, render_transform_system, audio_system, space_membership_system));
    }

    /// Expose mutable access to fixed schedule so callers can register systems.
    pub fn fixed_schedule_mut(&mut self) -> &mut Schedule {
        &mut self.fixed_schedule
    }

    /// Expose mutable access to variable schedule so callers can register systems.
    pub fn variable_schedule_mut(&mut self) -> &mut Schedule {
        &mut self.variable_schedule
    }

    pub fn create_entity(&mut self) -> Entity {
        self.world.spawn_empty().id()
    }

    pub fn destroy_entity(&mut self, e: Entity) {
        self.world.despawn(e);
    }

    pub fn add_sound(&mut self, entity: Entity, asset_id: impl Into<String>) {
        let src = AudioSourceComponent {
            asset_id: asset_id.into(),
            is_spatial: false,
            priority: 50,
            category: "SFX".to_string(),
        };
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            e.insert(src);
        }
    }

    pub fn add_stream(&mut self, entity: Entity, stream_id: u64, is_spatial: bool) {
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            e.insert(crate::components::AudioStreamComponent { stream_id, is_spatial });
        }
    }

    pub fn load_async(&mut self, asset_id: impl Into<String>) {
        let mut ev = self.world.resource_mut::<Events<LoadAssetEvent>>();
        ev.send(LoadAssetEvent { asset_id: asset_id.into() });
    }

    pub fn release_asset(&mut self, asset_id: impl Into<String>) {
        let mut ev = self.world.resource_mut::<Events<ReleaseAssetEvent>>();
        ev.send(ReleaseAssetEvent { asset_id: asset_id.into() });
    }

    pub fn open_audio_stream(&mut self, asset_id: impl Into<String>) {
        let mut ev = self.world.resource_mut::<Events<OpenAudioStreamEvent>>();
        ev.send(OpenAudioStreamEvent { asset_id: asset_id.into() });
    }

    pub fn close_audio_stream(&mut self, stream_id: u64) {
        let mut ev = self.world.resource_mut::<Events<CloseAudioStreamEvent>>();
        ev.send(CloseAudioStreamEvent { stream_id });
    }

    pub fn play(&mut self, entity: Entity) {
        let mut events = self.world.resource_mut::<Events<PlaySoundEvent>>();
        events.send(PlaySoundEvent { entity });
    }

    pub fn pause(&mut self, entity: Entity) {
        let mut events = self.world.resource_mut::<Events<PauseSoundEvent>>();
        events.send(PauseSoundEvent { entity });
    }

    pub fn stop(&mut self, entity: Entity) {
        let mut events = self.world.resource_mut::<Events<StopSoundEvent>>();
        events.send(StopSoundEvent { entity });
    }

    pub fn set_volume(&mut self, entity: Entity, volume: f32) {
        let mut events = self.world.resource_mut::<Events<SetVolumeEvent>>();
        events.send(SetVolumeEvent { entity, volume });
    }

    pub fn set_position(&mut self, entity: Entity, pos: glam::Vec3) {
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            if let Some(mut t) = e.get_mut::<TransformComponent>() {
                t.position = pos;
            } else {
                e.insert(TransformComponent { position: pos, rotation: glam::Quat::IDENTITY, scale: glam::Vec3::ONE, parent: None });
            }
        }
    }

    pub fn set_orientation(&mut self, entity: Entity, rot: glam::Quat) {
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            if let Some(mut t) = e.get_mut::<TransformComponent>() {
                t.rotation = rot;
            } else {
                e.insert(TransformComponent { position: glam::Vec3::ZERO, rotation: rot, scale: glam::Vec3::ONE, parent: None });
            }
        }
    }

    pub fn set_listener(&mut self, entity: Entity) {
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            e.insert(AudioListenerComponent);
        }
    }

    /// Runs fixed update then variable update once with the provided delta (seconds)
    pub fn update(&mut self, delta: f32) {
        // Provide TimeStep for variable systems
        self.world.insert_resource(crate::systems::TimeStep(delta.max(0.0)));
        self.fixed_schedule.run(&mut self.world);
        self.variable_schedule.run(&mut self.world);
        // advance events frame state at end, so EventReaders see events next frame
        crate::events::ensure_space_nav_events(&mut self.world);
        crate::events::update_event_resources(&mut self.world);
    }

    // --- Space creation APIs ---
    pub fn create_world_region(&mut self, name: impl Into<String>, min: glam::Vec3, max: glam::Vec3) -> Entity {
        let e = self.world.spawn(SpaceComponent { kind: SpaceKind::World, name: name.into(), bounds: Shape3D::Aabb(Aabb { min, max }), has_ceiling: true, medium: MediumType::Air }).id();
        e
    }

    pub fn create_city(&mut self, name: impl Into<String>, min: glam::Vec3, max: glam::Vec3) -> Entity {
        self.world.spawn(SpaceComponent { kind: SpaceKind::City, name: name.into(), bounds: Shape3D::Aabb(Aabb { min, max }), has_ceiling: false, medium: MediumType::Air }).id()
    }

    pub fn create_room(&mut self, name: impl Into<String>, min: glam::Vec3, max: glam::Vec3) -> Entity {
        self.world.spawn(SpaceComponent { kind: SpaceKind::Room, name: name.into(), bounds: Shape3D::Aabb(Aabb { min, max }), has_ceiling: true, medium: MediumType::Air }).id()
    }

    /// Create an ocean/water space (medium = Water)
    pub fn create_ocean(&mut self, name: impl Into<String>, min: glam::Vec3, max: glam::Vec3) -> Entity {
        self.world.spawn(SpaceComponent { kind: SpaceKind::Ocean, name: name.into(), bounds: Shape3D::Aabb(Aabb { min, max }), has_ceiling: false, medium: MediumType::Water }).id()
    }

    /// Create a free-form shaped space
    pub fn create_space_with_shape(&mut self, name: impl Into<String>, kind: SpaceKind, shape: Shape3D, has_ceiling: bool, medium: MediumType) -> Entity {
        self.world.spawn(SpaceComponent { kind, name: name.into(), bounds: shape, has_ceiling, medium }).id()
    }

    // --- Navigation APIs ---
    pub fn ensure_navigation(&mut self, entity: Entity, speed: f32) {
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            if e.get::<NavigationState>().is_none() {
                e.insert(NavigationState { target: None, speed });
            }
            if e.get::<TransformComponent>().is_none() {
                e.insert(TransformComponent { position: glam::Vec3::ZERO, rotation: glam::Quat::IDENTITY, scale: glam::Vec3::ONE, parent: None });
            }
            if e.get::<InsideSpaces>().is_none() {
                e.insert(InsideSpaces::default());
            }
            if e.get::<PreviousPosition>().is_none() {
                e.insert(PreviousPosition(glam::Vec3::ZERO));
            }
        }
    }

    pub fn navigate_to(&mut self, entity: Entity, target: glam::Vec3, speed: f32) {
        // Ensure event resource exists
        crate::events::ensure_space_nav_events(&mut self.world);
        let mut ev = self.world.resource_mut::<Events<NavigateToEvent>>();
        ev.send(NavigateToEvent { entity, target, speed });
    }

    /// Add or update traversal tags for an entity (used by portal allowlist)
    pub fn set_traversal_tags(&mut self, entity: Entity, tags: impl IntoIterator<Item=impl Into<String>>) {
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            let mut comp = if let Some(t) = e.get_mut::<TraversalTags>() { t.clone() } else { TraversalTags::default() };
            comp.tags = tags.into_iter().map(|s| s.into()).collect();
            e.insert(comp);
        }
    }

    /// Add a portal (door/window) connecting two spaces
    pub fn add_portal(&mut self, from: Entity, to: Entity, shape: Shape3D, bidirectional: bool, is_open: bool, allow_tags: Option<Vec<String>>) -> Entity {
        let allow = allow_tags.map(|v| v.into_iter().collect());
        self.world.spawn(PortalComponent { from, to, shape, bidirectional, is_open, allow_tags: allow }).id()
    }

    /// Toggle a portal open/closed
    pub fn set_portal_open(&mut self, portal_entity: Entity, open: bool) {
        if let Some(mut e) = self.world.get_entity_mut(portal_entity) {
            if let Some(mut p) = e.get_mut::<PortalComponent>() { p.is_open = open; }
        }
    }

    /// Grant climbing ability
    pub fn grant_climb(&mut self, entity: Entity) {
        if let Some(mut e) = self.world.get_entity_mut(entity) { e.insert(CanClimb); }
    }
    /// Grant diving ability
    pub fn grant_dive(&mut self, entity: Entity) {
        if let Some(mut e) = self.world.get_entity_mut(entity) { e.insert(CanDive); }
    }
}
