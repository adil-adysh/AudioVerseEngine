use bevy_ecs::prelude::*;
use rapier3d::prelude::*;
use std::sync::Mutex;
use std::collections::HashMap;

use crate::components::{PhysicsColliderComponent, PhysicsComponent, PhysicsHandle, TransformComponent, WorldTransformComponent};
use crate::events::PhysicsCollisionEvent;
/// Rapier-backed physics resources using rapier3d v0.23 types.
#[derive(Resource)]
pub struct PhysicsResources {
    pub physics_pipeline: PhysicsPipeline,
    pub gravity: Vector<Real>,
    pub integration_parameters: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
}

impl Default for PhysicsResources {
    fn default() -> Self {
        Self {
            physics_pipeline: PhysicsPipeline::new(),
            gravity: vector![0.0, -9.81, 0.0],
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }
}

/// Spawn system: create Rapier rigid bodies and colliders for entities that
/// have a `PhysicsComponent` and don't yet have a `PhysicsHandle`.
#[allow(clippy::type_complexity)]
pub fn physics_spawn_system(mut commands: Commands, mut phys: ResMut<PhysicsResources>, query: Query<(Entity, &PhysicsComponent, Option<&PhysicsColliderComponent>, Option<&PhysicsHandle>, &TransformComponent)>) {
    // First pass: collect spawn requests without mutably borrowing `phys`.
    struct SpawnRequest {
        entity: Entity,
        pc: PhysicsComponent,
        collider: Option<PhysicsColliderComponent>,
        transform: TransformComponent,
    }

    let mut requests: Vec<SpawnRequest> = Vec::new();
    for (entity, pc, collider_opt, handle_opt, transform) in query.iter() {
        if handle_opt.is_some() {
            continue;
        }

        requests.push(SpawnRequest {
            entity,
            pc: pc.clone(),
            collider: collider_opt.cloned(),
            transform: transform.clone(),
        });
    }

    if requests.is_empty() {
        return;
    }

    // Build Rapier objects first without borrowing `phys`.
    let mut to_insert: Vec<(Entity, RigidBody, Option<Collider>)> = Vec::new();
    for req in requests.into_iter() {
        let mut rb_builder = match req.pc.body_type {
            0 => RigidBodyBuilder::fixed(),
            1 => RigidBodyBuilder::kinematic_position_based(),
            _ => RigidBodyBuilder::dynamic(),
        };

        rb_builder = rb_builder
            .translation(vector![req.transform.position.x, req.transform.position.y, req.transform.position.z]);

        let rb = rb_builder.build();

        let col = req.collider.map(|col_comp| match col_comp.shape {
            0 => ColliderBuilder::ball(col_comp.shape_params[0]).build(),
            1 => ColliderBuilder::cuboid(col_comp.shape_params[0], col_comp.shape_params[1], col_comp.shape_params[2]).build(),
            2 => ColliderBuilder::capsule_y(col_comp.shape_params[0], col_comp.shape_params[1]).build(),
            _ => ColliderBuilder::ball(0.5).build(),
        });

        to_insert.push((req.entity, rb, col));
    }

    // Now do a single mutable borrow to insert all rigid bodies into Rapier sets.
    // Colliders are deferred for a follow-up to keep borrowing simple.
    {
        let PhysicsResources { rigid_body_set, collider_set, .. } = &mut *phys;
        for (entity, rb, col_opt) in to_insert.into_iter() {
            let rb_handle = rigid_body_set.insert(rb);
            let col_handle = col_opt.map(|col| collider_set.insert_with_parent(col, rb_handle, rigid_body_set));

            commands
                .entity(entity)
                .insert(PhysicsHandle { rb: Some(rb_handle), col: col_handle });
        }
    }
}

/// Simple collector that forwards Rapier contact events to a local buffer so we can
/// translate them into Bevy `PhysicsCollisionEvent`s after stepping the pipeline.
struct ContactEventCollector {
    // (collider1, collider2, started)
    pairs: Mutex<Vec<(ColliderHandle, ColliderHandle, bool)>>,
}

impl ContactEventCollector {
    fn new() -> Self { Self { pairs: Mutex::new(Vec::new()) } }
}

impl EventHandler for ContactEventCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        match event {
            CollisionEvent::Started(h1, h2, _flags) => {
                if let Ok(mut v) = self.pairs.lock() { v.push((h1, h2, true)); }
            }
            CollisionEvent::Stopped(h1, h2, _flags) => {
                if let Ok(mut v) = self.pairs.lock() { v.push((h1, h2, false)); }
            }
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _pair: &ContactPair,
        _total_force_magnitude: Real,
    ) {
        // Optional: could accumulate impulses and relative velocities later.
    }
}

/// Step the physics pipeline and write back transforms for entities that
/// have Rapier handles. Emits `PhysicsCollisionEvent` when collisions are
/// observed via the event handler (here we use an empty handler for now).
pub fn physics_step_system(
    mut phys: ResMut<PhysicsResources>,
    mut events: ResMut<Events<PhysicsCollisionEvent>>,
    mut query: Query<(Entity, &PhysicsHandle, Option<&mut WorldTransformComponent>)>,
    pc_query: Query<&PhysicsComponent>,
) {
    // Step Rapier (use unit hooks and unit event handler for now).
    let physics_hooks = ();
    let event_handler = ContactEventCollector::new();

    // Destructure `phys` to get mutable references to Rapier sets and params.
    let PhysicsResources { physics_pipeline, island_manager, broad_phase, narrow_phase, rigid_body_set, collider_set, impulse_joint_set, multibody_joint_set, ccd_solver, query_pipeline, gravity, integration_parameters, .. } = &mut *phys;

    // IntegrationParameters and gravity implement Copy, so pass by value.
    physics_pipeline.step(
        gravity,
        integration_parameters,
        island_manager,
        broad_phase,
        narrow_phase,
        rigid_body_set,
        collider_set,
        impulse_joint_set,
        multibody_joint_set,
        ccd_solver,
        Some(query_pipeline),
        &physics_hooks,
        &event_handler,
    );

    // After stepping, use the local `rigid_body_set` reference (no extra borrows of `phys`).
    for (_entity, handle, wt_opt) in query.iter_mut() {
        if let Some(rb_h) = handle.rb {
            if let Some(rb) = rigid_body_set.get(rb_h) {
                let pos = rb.position().translation.vector;
                if let Some(mut wt) = wt_opt {
                    wt.matrix = glam::Mat4::from_translation(glam::Vec3::new(pos.x, pos.y, pos.z));
                }
            }
        }
    }

    // Build a mapping from ColliderHandle -> Entity for event translation.
    let mut col_to_entity: HashMap<ColliderHandle, Entity> = HashMap::new();
    for (entity, handle, _) in query.iter_mut() {
        if let Some(ch) = handle.col {
            col_to_entity.insert(ch, entity);
        }
    }

    // Drain collected contact events and publish Bevy events.
    let drained_pairs: Vec<(ColliderHandle, ColliderHandle, bool)> = if let Ok(mut buf) = event_handler.pairs.lock() {
        buf.drain(..).collect()
    } else { Vec::new() };
    for (h1, h2, started) in drained_pairs.into_iter() {
        if !started { continue; }
        let (entity_a_opt, entity_b_opt) = (col_to_entity.get(&h1).copied(), col_to_entity.get(&h2).copied());
        if let (Some(entity_a), Some(entity_b)) = (entity_a_opt, entity_b_opt) {
            // Try to fetch material profiles; fall back to empty strings if missing.
            let mat_a = pc_query.get(entity_a).map(|pc| pc.material_profile.clone()).unwrap_or_default();
            let mat_b = pc_query.get(entity_b).map(|pc| pc.material_profile.clone()).unwrap_or_default();

            events.send(PhysicsCollisionEvent {
                entity_a,
                entity_b,
                // Detailed contact info requires querying manifolds; use placeholders for now.
                contact_point: [0.0, 0.0, 0.0],
                relative_velocity: [0.0, 0.0, 0.0],
                impulse: 0.0,
                materials: (mat_a, mat_b),
            });
        }
    }
}
