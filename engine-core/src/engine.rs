use bevy_app::Update;
use bevy_ecs::prelude::*;
use bevy_time::TimePlugin;

use crate::components::NavigationPath;
use crate::components::*;
use crate::events::{
    CloseAudioStreamEvent, LoadAssetEvent, NavigateToEvent, OpenAudioStreamEvent, PauseSoundEvent,
    PlaySoundEvent, ReleaseAssetEvent, SetVolumeEvent, StopSoundEvent,
};
use crate::navmesh::NavMesh;
use crate::systems::{
    audio_listener_system, audio_system, navigation_step_system, navigation_system,
    set_update_timestep_system, space_membership_system,
};
use crate::transform::*;

pub struct Engine {
    app: bevy_app::App,
}

impl Engine {
    pub fn new() -> Self {
        let mut app = bevy_app::App::new();
        // Time plugin for delta tracking (we still pass dt manually via TimeStep)
        app.add_plugins(TimePlugin);

        // Register baseline resources/events
        crate::events::init_event_resources(&mut app.world);
        if app
            .world
            .get_resource::<crate::components::SpaceGraph>()
            .is_none()
        {
            app.world
                .insert_resource(crate::components::SpaceGraph::default());
        }
        // Physics resources
        if app
            .world
            .get_resource::<crate::physics::PhysicsResources>()
            .is_none()
        {
            app.world
                .insert_resource(crate::physics::PhysicsResources::default());
        }

        // Wire all engine systems into the Bevy Update schedule in deterministic order.
    use bevy_ecs::schedule::IntoSystemConfigs;
    // Split long chains into smaller chained groups to satisfy IntoSystemConfigs bounds
    app.add_systems(Update, (set_update_timestep_system, navigation_system, physics_transform_system).chain());
    app.add_systems(Update, (navigation_step_system, crate::physics::physics_spawn_system, crate::physics::physics_step_system).chain());
    app.add_systems(Update, (render_transform_system, audio_listener_system, audio_system).chain());
    app.add_systems(Update, (crate::transform::despawn_cleanup_system, crate::systems::space_graph_index_system).chain());
    app.add_systems(Update, (crate::systems::navmesh_boundary_cues_system, crate::systems::navmesh_wayfinding_cues_system, space_membership_system).chain());

        Self { app }
    }
}

impl Engine {
    /// Called once to register engine-provided systems and do one-time setup.
    pub fn bootstrap(&mut self) {
        // No-op: systems are registered during Engine::new() into the App.
    }

    /// Access Bevy App for advanced wiring (e.g., adding systems/plugins).
    pub fn app_mut(&mut self) -> &mut bevy_app::App { &mut self.app }

    /// Access to the ECS world (immutable).
    pub fn world(&self) -> &World { &self.app.world }

    /// Access to the ECS world (mutable).
    pub fn world_mut(&mut self) -> &mut World { &mut self.app.world }

    pub fn create_entity(&mut self) -> Entity {
    self.world_mut().spawn_empty().id()
    }

    pub fn destroy_entity(&mut self, e: Entity) {
    self.world_mut().despawn(e);
    }

    pub fn add_sound(&mut self, entity: Entity, asset_id: impl Into<String>) {
        let src = AudioSourceComponent {
            asset_id: asset_id.into(),
            is_spatial: false,
            priority: 50,
            category: "SFX".to_string(),
        };
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            e.insert(src);
        }
    }

    pub fn add_stream(&mut self, entity: Entity, stream_id: u64, is_spatial: bool) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            e.insert(crate::components::AudioStreamComponent {
                stream_id,
                is_spatial,
            });
        }
    }

    pub fn load_async(&mut self, asset_id: impl Into<String>) {
    let mut ev = self.app.world.resource_mut::<Events<LoadAssetEvent>>();
        ev.send(LoadAssetEvent {
            asset_id: asset_id.into(),
        });
    }

    pub fn release_asset(&mut self, asset_id: impl Into<String>) {
    let mut ev = self.app.world.resource_mut::<Events<ReleaseAssetEvent>>();
        ev.send(ReleaseAssetEvent {
            asset_id: asset_id.into(),
        });
    }

    pub fn open_audio_stream(&mut self, asset_id: impl Into<String>) {
    let mut ev = self.app.world.resource_mut::<Events<OpenAudioStreamEvent>>();
        ev.send(OpenAudioStreamEvent {
            asset_id: asset_id.into(),
        });
    }

    pub fn close_audio_stream(&mut self, stream_id: u64) {
    let mut ev = self.app.world.resource_mut::<Events<CloseAudioStreamEvent>>();
        ev.send(CloseAudioStreamEvent { stream_id });
    }

    pub fn play(&mut self, entity: Entity) {
    let mut events = self.app.world.resource_mut::<Events<PlaySoundEvent>>();
        events.send(PlaySoundEvent { entity });
    }

    pub fn pause(&mut self, entity: Entity) {
    let mut events = self.app.world.resource_mut::<Events<PauseSoundEvent>>();
        events.send(PauseSoundEvent { entity });
    }

    pub fn stop(&mut self, entity: Entity) {
    let mut events = self.app.world.resource_mut::<Events<StopSoundEvent>>();
        events.send(StopSoundEvent { entity });
    }

    pub fn set_volume(&mut self, entity: Entity, volume: f32) {
    let mut events = self.app.world.resource_mut::<Events<SetVolumeEvent>>();
        events.send(SetVolumeEvent { entity, volume });
    }

    pub fn set_position(&mut self, entity: Entity, pos: glam::Vec3) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            if let Some(mut t) = e.get_mut::<TransformComponent>() {
                t.position = pos;
            } else {
                e.insert(TransformComponent {
                    position: pos,
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                    parent: None,
                });
            }
        }
    }

    pub fn set_orientation(&mut self, entity: Entity, rot: glam::Quat) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            if let Some(mut t) = e.get_mut::<TransformComponent>() {
                t.rotation = rot;
            } else {
                e.insert(TransformComponent {
                    position: glam::Vec3::ZERO,
                    rotation: rot,
                    scale: glam::Vec3::ONE,
                    parent: None,
                });
            }
        }
    }

    pub fn set_listener(&mut self, entity: Entity) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            e.insert(AudioListenerComponent);
        }
    }

    /// Runs fixed update then variable update once with the provided delta (seconds)
    pub fn update(&mut self, delta: f32) {
        // Provide TimeStep for systems as real frame delta; the first system in Update will also set this.
        let dt_frame = delta.max(0.0);
        self.world_mut()
            .insert_resource(crate::systems::TimeStep(dt_frame));

        // Ensure navigation/space event resources exist
    crate::events::ensure_space_nav_events(self.world_mut());

        // Run Bevy App once
    self.app.update();
    // Advance events at end of frame
    crate::events::update_event_resources(self.world_mut());
    }

    // --- Space creation APIs ---
    pub fn create_world_region(
        &mut self,
        name: impl Into<String>,
        min: glam::Vec3,
        max: glam::Vec3,
    ) -> Entity {
        let e = self
            .app
            .world
            .spawn(SpaceComponent {
                kind: SpaceKind::World,
                name: name.into(),
                bounds: Shape3D::Aabb(Aabb { min, max }),
                has_ceiling: true,
                medium: MediumType::Air,
            })
            .id();
        e
    }

    pub fn create_city(
        &mut self,
        name: impl Into<String>,
        min: glam::Vec3,
        max: glam::Vec3,
    ) -> Entity {
    self.app.world
            .spawn(SpaceComponent {
                kind: SpaceKind::City,
                name: name.into(),
                bounds: Shape3D::Aabb(Aabb { min, max }),
                has_ceiling: false,
                medium: MediumType::Air,
            })
            .id()
    }

    pub fn create_room(
        &mut self,
        name: impl Into<String>,
        min: glam::Vec3,
        max: glam::Vec3,
    ) -> Entity {
    self.app.world
            .spawn(SpaceComponent {
                kind: SpaceKind::Room,
                name: name.into(),
                bounds: Shape3D::Aabb(Aabb { min, max }),
                has_ceiling: true,
                medium: MediumType::Air,
            })
            .id()
    }

    /// Create an ocean/water space (medium = Water)
    pub fn create_ocean(
        &mut self,
        name: impl Into<String>,
        min: glam::Vec3,
        max: glam::Vec3,
    ) -> Entity {
    self.app.world
            .spawn(SpaceComponent {
                kind: SpaceKind::Ocean,
                name: name.into(),
                bounds: Shape3D::Aabb(Aabb { min, max }),
                has_ceiling: false,
                medium: MediumType::Water,
            })
            .id()
    }

    /// Create a free-form shaped space
    pub fn create_space_with_shape(
        &mut self,
        name: impl Into<String>,
        kind: SpaceKind,
        shape: Shape3D,
        has_ceiling: bool,
        medium: MediumType,
    ) -> Entity {
    self.app.world
            .spawn(SpaceComponent {
                kind,
                name: name.into(),
                bounds: shape,
                has_ceiling,
                medium,
            })
            .id()
    }

    /// Create a generic space with tags and optional medium
    pub fn create_space_generic(
        &mut self,
        name: impl Into<String>,
        shape: Shape3D,
        tags: &[&str],
        medium: Option<MediumType>,
    ) -> Entity {
        // Compute tag mask before spawning to avoid overlapping borrows
        if !self
            .app
            .world
            .contains_resource::<crate::components::TagRegistry>()
        {
            self.app
                .world
                .insert_resource(crate::components::TagRegistry::default());
        }
        let mask = {
            let mut reg = self
                .app
                .world
                .resource_mut::<crate::components::TagRegistry>();
            reg.mask_for(tags.iter().copied())
        };
        self.app.world
            .spawn((
                SpaceComponent {
                    kind: SpaceKind::Zone,
                    name: name.into(),
                    bounds: shape,
                    has_ceiling: true,
                    medium: medium.unwrap_or(MediumType::Air),
                },
                crate::components::SpaceTags { mask },
            ))
            .id()
    }

    // --- Navigation APIs ---
    pub fn ensure_navigation(&mut self, entity: Entity, speed: f32) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            if e.get::<NavigationState>().is_none() {
                e.insert(NavigationState {
                    target: None,
                    speed,
                });
            }
            if e.get::<TransformComponent>().is_none() {
                e.insert(TransformComponent {
                    position: glam::Vec3::ZERO,
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                    parent: None,
                });
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
        crate::events::ensure_space_nav_events(&mut self.app.world);
        let mut ev = self
            .app
            .world
            .resource_mut::<Events<NavigateToEvent>>();
        ev.send(NavigateToEvent {
            entity,
            target,
            speed,
        });
    }

    /// Compute a space-to-space path and follow it; falls back to straight target if no path.
    pub fn navigate_to_space(
        &mut self,
        entity: Entity,
        start_space: Entity,
        goal_space: Entity,
        speed: f32,
    ) {
        // ensure pathfinding module is available
        if self
            .app
            .world
            .get_resource::<crate::components::SpaceGraph>()
            .is_some()
        {
            let graph = self
                .app
                .world
                .resource::<crate::components::SpaceGraph>()
                .clone();
            let mut spaces_q = self
                .app
                .world
                .query::<(Entity, &crate::components::SpaceComponent)>();
            let mut portals_q = self
                .app
                .world
                .query::<(Entity, &crate::components::PortalComponent)>();
            // Snapshot spaces and portals; queries outside systems require copying data we need.
            let mut spaces: Vec<(Entity, crate::components::SpaceComponent)> = Vec::new();
            for (e, sc) in spaces_q.iter(&self.app.world) {
                spaces.push((e, sc.clone()));
            }
            let mut portals: Vec<(Entity, crate::components::PortalComponent)> = Vec::new();
            for (e, p) in portals_q.iter(&self.app.world) {
                portals.push((e, p.clone()));
            }

            // Helper closures from snapshots
            let center_of = |e: Entity| -> glam::Vec3 {
                spaces
                    .iter()
                    .find(|(id, _)| *id == e)
                    .map(|(_, sc)| sc.center())
                    .unwrap_or(glam::Vec3::ZERO)
            };
            let neighbors_of = |e: Entity| -> Vec<Entity> {
                let mut out = Vec::new();
                if let Some(v) = graph.portals_from.get(&e) {
                    for pe in v {
                        if let Some((_id, p)) = portals.iter().find(|(id, _)| id == pe) {
                            out.push(p.to);
                        }
                    }
                }
                if let Some(v) = graph.portals_to.get(&e) {
                    for pe in v {
                        if let Some((_id, p)) = portals.iter().find(|(id, _)| id == pe) {
                            out.push(p.from);
                        }
                    }
                }
                out
            };

            if let Some(waypoints) = crate::pathfinding::astar_spaces(
                &graph,
                center_of,
                neighbors_of,
                start_space,
                goal_space,
            ) {
                if let Some(mut ent) = self.app.world.get_entity_mut(entity) {
                    ent.insert(NavigationPath {
                        waypoints: waypoints.clone(),
                        index: 0,
                    });
                }
                if let Some(mut e) = self.app.world.get_entity_mut(entity) {
                    if e.get::<crate::components::NavigationState>().is_none() {
                        e.insert(crate::components::NavigationState {
                            target: None,
                            speed,
                        });
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
        if let Some((_e, sc)) = self
            .app
            .world
            .query::<(Entity, &crate::components::SpaceComponent)>()
            .iter(&self.app.world)
            .find(|(e, _)| *e == goal_space)
        {
            self.navigate_to(entity, sc.center(), speed);
        }
    }

    /// Add or update traversal tags for an entity (used by portal allowlist)
    pub fn set_traversal_tags(
        &mut self,
        entity: Entity,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let mut reg = if self.app.world.contains_resource::<TagRegistry>() {
            self.app.world.resource_mut::<TagRegistry>()
        } else {
            self.app.world.insert_resource(TagRegistry::default());
            self.app.world.resource_mut::<TagRegistry>()
        };
        let collected: Vec<String> = tags.into_iter().map(|s| s.into()).collect();
        let mask = reg.mask_for(collected.iter().map(|s| s.as_str()));
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            e.insert(TraversalMask { mask });
        }
    }

    /// Add a portal (door/window) connecting two spaces
    pub fn add_portal(
        &mut self,
        from: Entity,
        to: Entity,
        shape: Shape3D,
        bidirectional: bool,
        is_open: bool,
        allow_tags: Option<Vec<String>>,
    ) -> Entity {
        let mut allow_mask = 0u64;
        if let Some(tags) = allow_tags {
            let mut reg = if self.app.world.contains_resource::<TagRegistry>() {
                self.app.world.resource_mut::<TagRegistry>()
            } else {
                self.app.world.insert_resource(TagRegistry::default());
                self.app.world.resource_mut::<TagRegistry>()
            };
            allow_mask = reg.mask_for(tags.iter().map(|s| s.as_str()));
        }
        self.app.world
            .spawn(PortalComponent {
                from,
                to,
                shape,
                bidirectional,
                is_open,
                allow_mask,
                required_abilities_mask: 0,
                cost: 1.0,
            })
            .id()
    }

    /// Add a portal with a richer traversal policy (tags + required abilities + cost)
    #[allow(clippy::too_many_arguments)]
    pub fn add_portal_with_policy(
        &mut self,
        from: Entity,
        to: Entity,
        shape: Shape3D,
        bidirectional: bool,
        is_open: bool,
        allow_tags: &[&str],
        required_abilities: &[&str],
        cost: f32,
    ) -> Entity {
        if !self.app.world.contains_resource::<TagRegistry>() {
            self.app.world.insert_resource(TagRegistry::default());
        }
        if !self
            .app
            .world
            .contains_resource::<crate::components::AbilityRegistry>()
        {
            self.app
                .world
                .insert_resource(crate::components::AbilityRegistry::default());
        }
        let allow_mask = {
            let mut tag_reg = self.app.world.resource_mut::<TagRegistry>();
            tag_reg.mask_for(allow_tags.iter().copied())
        };
        let req_mask = {
            let mut abil_reg = self
                .app
                .world
                .resource_mut::<crate::components::AbilityRegistry>();
            abil_reg.mask_for(required_abilities.iter().copied())
        };
        self.app.world
            .spawn(PortalComponent {
                from,
                to,
                shape,
                bidirectional,
                is_open,
                allow_mask,
                required_abilities_mask: req_mask,
                cost,
            })
            .id()
    }

    /// Toggle a portal open/closed
    pub fn set_portal_open(&mut self, portal_entity: Entity, open: bool) {
    if let Some(mut e) = self.app.world.get_entity_mut(portal_entity) {
            if let Some(mut p) = e.get_mut::<PortalComponent>() {
                p.is_open = open;
            }
        }
    }

    /// Grant climbing ability
    pub fn grant_climb(&mut self, entity: Entity) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            e.insert(CanClimb);
        }
    }
    /// Grant diving ability
    pub fn grant_dive(&mut self, entity: Entity) {
    if let Some(mut e) = self.app.world.get_entity_mut(entity) {
            e.insert(CanDive);
        }
    }

    // --- NavMesh APIs ---
    /// Build a simple navmesh from axis-aligned rectangles in XZ plane. Y is ignored.
    pub fn set_navmesh_rects(&mut self, rects: &[(f32, f32, f32, f32)]) {
        let mesh = NavMesh::from_rects(rects);
    self.app.world.insert_resource(mesh);
    }

    /// Enable navmesh audio cues on an entity with thresholds.
    pub fn enable_navmesh_cues(
        &mut self,
        entity: Entity,
        boundary_warn_distance: f32,
        turn_cue_angle_deg: f32,
    ) {
    if let Some(mut emut) = self.app.world.get_entity_mut(entity) {
            emut.insert(crate::components::NavmeshGuidance {
                boundary_warn_distance,
                turn_cue_angle_deg,
            });
        }
    }
}

impl Default for Engine {
    fn default() -> Self { Self::new() }
}

