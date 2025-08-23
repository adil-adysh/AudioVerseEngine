use bevy_ecs::prelude::*;

use crate::components::*;
use crate::events::{
    PlaySoundEvent, StopSoundEvent, PauseSoundEvent, SetVolumeEvent, LoadAssetEvent,
    ReleaseAssetEvent, OpenAudioStreamEvent, CloseAudioStreamEvent,
    NavigateToEvent,
};
use crate::systems::{
    audio_listener_system, audio_system, navigation_system, navigation_step_system,
    space_membership_system,
};
use crate::transform::*;
use crate::components::NavigationPath;
use crate::navmesh::NavMesh;

pub struct Engine {
    pub world: World,
    pub fixed_schedule: Schedule,
    pub variable_schedule: Schedule,
}

impl Engine {
    pub fn new() -> Self {
    let mut world = World::new();
    crate::events::init_event_resources(&mut world);

    let fixed_schedule = Schedule::default();
    let variable_schedule = Schedule::default();

    // default fixed-step config (60 Hz)
    world.insert_resource(FixedStepConfig { dt: 1.0 / 60.0, accumulator: 0.0, max_substeps: 5 });

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

    // add transforms, navigation step, and physics systems to fixed schedule
    self.fixed_schedule.add_systems((physics_transform_system, navigation_step_system, physics_spawn_system, physics_step_system));

    // ensure SpaceGraph resource exists and indexer runs each frame before membership
    if self.world.get_resource::<crate::components::SpaceGraph>().is_none() {
        self.world.insert_resource(crate::components::SpaceGraph::default());
    }

    // variable schedule: navigation setup, listener, render transforms, audio, navmesh cues, space membership
    self.variable_schedule.add_systems((
        navigation_system,
        audio_listener_system,
        render_transform_system,
        audio_system,
        crate::systems::space_graph_index_system,
        crate::systems::navmesh_boundary_cues_system,
        crate::systems::navmesh_wayfinding_cues_system,
        space_membership_system,
    ));
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
        // Accumulate time and run fixed steps with constant dt
        let dt_frame = delta.max(0.0);
        // determine number of fixed steps and dt without holding borrow during runs
        let (steps, dt_fixed) = {
            let mut cfg = self.world.resource_mut::<FixedStepConfig>();
            cfg.accumulator += dt_frame;
            let mut steps = (cfg.accumulator / cfg.dt).floor() as u32;
            if steps > cfg.max_substeps { steps = cfg.max_substeps; }
            let dt_fixed = cfg.dt;
            (steps, dt_fixed)
        };
        if steps > 0 {
            for _ in 0..steps {
                // Provide fixed TimeStep value for fixed systems
                self.world.insert_resource(crate::systems::TimeStep(dt_fixed));
                self.fixed_schedule.run(&mut self.world);
            }
            // subtract consumed time
            let mut cfg = self.world.resource_mut::<FixedStepConfig>();
            cfg.accumulator -= dt_fixed * steps as f32;
        }

        // Provide TimeStep for variable systems as real frame delta
        self.world.insert_resource(crate::systems::TimeStep(dt_frame));
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

    /// Compute a space-to-space path and follow it; falls back to straight target if no path.
    pub fn navigate_to_space(&mut self, entity: Entity, start_space: Entity, goal_space: Entity, speed: f32) {
        // ensure pathfinding module is available
        if self.world.get_resource::<crate::components::SpaceGraph>().is_some() {
            let graph = self.world.resource::<crate::components::SpaceGraph>().clone();
            let mut spaces_q = self.world.query::<(Entity, &crate::components::SpaceComponent)>();
            let mut portals_q = self.world.query::<(Entity, &crate::components::PortalComponent)>();
            // Snapshot spaces and portals; queries outside systems require copying data we need.
            let mut spaces: Vec<(Entity, crate::components::SpaceComponent)> = Vec::new();
            for (e, sc) in spaces_q.iter(&self.world) { spaces.push((e, sc.clone())); }
            let mut portals: Vec<(Entity, crate::components::PortalComponent)> = Vec::new();
            for (e, p) in portals_q.iter(&self.world) { portals.push((e, p.clone())); }

            // Helper closures from snapshots
            let center_of = |e: Entity| -> glam::Vec3 {
                spaces.iter().find(|(id, _)| *id == e).map(|(_, sc)| sc.center()).unwrap_or(glam::Vec3::ZERO)
            };
            let neighbors_of = |e: Entity| -> Vec<Entity> {
                let mut out = Vec::new();
                if let Some(v) = graph.portals_from.get(&e) {
                    for pe in v { if let Some((_id, p)) = portals.iter().find(|(id, _)| id == pe) { out.push(p.to); } }
                }
                if let Some(v) = graph.portals_to.get(&e) {
                    for pe in v { if let Some((_id, p)) = portals.iter().find(|(id, _)| id == pe) { out.push(p.from); } }
                }
                out
            };

            if let Some(waypoints) = crate::pathfinding::astar_spaces(&graph, center_of, neighbors_of, start_space, goal_space) {
                if let Some(mut ent) = self.world.get_entity_mut(entity) {
                    ent.insert(NavigationPath { waypoints: waypoints.clone(), index: 0 });
                }
                if let Some(mut e) = self.world.get_entity_mut(entity) {
                    if e.get::<crate::components::NavigationState>().is_none() {
                        e.insert(crate::components::NavigationState { target: None, speed });
                    }
                }
                // set first target
                if let Some(first) = waypoints.first().cloned() {
                    self.navigate_to(entity, first, speed);
                    return;
                }
            }
        }
        // fallback: no graph/path, just direct navigate
        self.ensure_navigation(entity, speed);
        // direct target is the goal space center
    if let Some((_e, sc)) = self.world.query::<(Entity, &crate::components::SpaceComponent)>().iter(&self.world).find(|(e, _)| *e == goal_space) {
            self.navigate_to(entity, sc.center(), speed);
        }
    }

    /// Add or update traversal tags for an entity (used by portal allowlist)
    pub fn set_traversal_tags(&mut self, entity: Entity, tags: impl IntoIterator<Item=impl Into<String>>) {
        let mut reg = if self.world.contains_resource::<TagRegistry>() { self.world.resource_mut::<TagRegistry>() } else { self.world.insert_resource(TagRegistry::default()); self.world.resource_mut::<TagRegistry>() };
        let collected: Vec<String> = tags.into_iter().map(|s| s.into()).collect();
        let mask = reg.mask_for(collected.iter().map(|s| s.as_str()));
        if let Some(mut e) = self.world.get_entity_mut(entity) {
            e.insert(TraversalMask { mask });
        }
    }

    /// Add a portal (door/window) connecting two spaces
    pub fn add_portal(&mut self, from: Entity, to: Entity, shape: Shape3D, bidirectional: bool, is_open: bool, allow_tags: Option<Vec<String>>) -> Entity {
        let mut allow_mask = 0u64;
        if let Some(tags) = allow_tags {
            let mut reg = if self.world.contains_resource::<TagRegistry>() { self.world.resource_mut::<TagRegistry>() } else { self.world.insert_resource(TagRegistry::default()); self.world.resource_mut::<TagRegistry>() };
            allow_mask = reg.mask_for(tags.iter().map(|s| s.as_str()));
        }
        self.world.spawn(PortalComponent { from, to, shape, bidirectional, is_open, allow_mask }).id()
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

    // --- NavMesh APIs ---
    /// Build a simple navmesh from axis-aligned rectangles in XZ plane. Y is ignored.
    pub fn set_navmesh_rects(&mut self, rects: &[(f32,f32,f32,f32)]) {
        let mesh = NavMesh::from_rects(rects);
        self.world.insert_resource(mesh);
    }

    /// Enable navmesh audio cues on an entity with thresholds.
    pub fn enable_navmesh_cues(&mut self, entity: Entity, boundary_warn_distance: f32, turn_cue_angle_deg: f32) {
        if let Some(mut emut) = self.world.get_entity_mut(entity) {
            emut.insert(crate::components::NavmeshGuidance { boundary_warn_distance, turn_cue_angle_deg });
        }
    }
}

/// Fixed-step configuration for the engine's fixed schedule
#[derive(Resource, Debug, Clone, Copy)]
pub struct FixedStepConfig {
    pub dt: f32,
    pub accumulator: f32,
    pub max_substeps: u32,
}

impl Engine {
    /// Set the fixed-step delta time (seconds) and optional max substeps per frame
    pub fn set_fixed_dt(&mut self, dt: f32, max_substeps: Option<u32>) {
        let mut cfg = self.world.resource_mut::<FixedStepConfig>();
        cfg.dt = dt.max(1e-6);
        if let Some(ms) = max_substeps { cfg.max_substeps = ms.max(1); }
    }
}
