use bevy_ecs::prelude::*;
use rapier3d::prelude::{RigidBodyHandle, ColliderHandle};
use std::collections::HashSet;

/// Local transform component (position, rotation, scale, parent)
#[derive(Component, Debug, Clone)]
pub struct TransformComponent {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
    pub parent: Option<Entity>,
}

/// Cached world transform
#[derive(Component, Debug, Clone)]
pub struct WorldTransformComponent {
    pub matrix: glam::Mat4,
}

/// Marker component for the audio listener
#[derive(Component, Debug, Clone, Default)]
pub struct AudioListenerComponent;

/// Audio source static configuration
#[derive(Component, Debug, Clone)]
pub struct AudioSourceComponent {
    pub asset_id: String,
    pub is_spatial: bool,
    // spatial options left intentionally simple for now
    pub priority: u8,
    pub category: String,
}

/// Runtime playback state
#[derive(Component, Debug)]
pub struct AudioPlaybackStateComponent {
    pub bus_name: String,
    pub is_spatial: bool,
    pub volume: f32,
    // placeholder for internal handle
    pub sound_instance_handle: Option<u64>,
}

/// Streaming audio source marker and configuration
#[derive(Component, Debug, Clone)]
pub struct AudioStreamComponent {
    pub stream_id: u64,
    pub is_spatial: bool,
}

/// Physics component: static configuration for a physics body
#[derive(Component, Debug, Clone)]
pub struct PhysicsComponent {
    pub mass: f32,
    pub is_trigger: bool,
    pub material_profile: String,
    /// body type: 0 = Static, 1 = Kinematic, 2 = Dynamic
    pub body_type: u8,
    pub restitution: f32,
    pub friction: f32,
}

/// Simple rigid-body runtime state stored on entities (optional)
#[derive(Component, Debug, Clone, Default)]
pub struct RigidBodyState {
    pub linear_velocity: glam::Vec3,
    pub angular_velocity: glam::Vec3,
    pub awake: bool,
}

/// Collider definition component (simple shapes for now)
#[derive(Component, Debug, Clone)]
pub struct PhysicsColliderComponent {
    /// shape enum as a small tag; 0=Ball,1=Box,2=Capsule,3=Mesh
    pub shape: u8,
    pub shape_params: [f32; 4],
    pub local_offset: glam::Vec3,
    pub local_rotation: glam::Quat,
}

/// Rapier handles stored on entities so we can map between ECS entities
/// and Rapier runtime objects.
#[derive(Component, Debug, Clone, Default)]
pub struct PhysicsHandle {
    pub rb: Option<RigidBodyHandle>,
    pub col: Option<ColliderHandle>,
}

/// Spatial hierarchy kinds (spaces): world, city, room, ocean, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpaceKind { World, City, Room, Zone, Ocean }

/// Axis-aligned bounding box used to define space extents
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

impl Aabb {
    pub fn contains(&self, p: glam::Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.y >= self.min.y && p.y <= self.max.y &&
        p.z >= self.min.z && p.z <= self.max.z
    }
}

/// Oriented bounding box
#[derive(Debug, Clone, Copy)]
pub struct Obb {
    pub center: glam::Vec3,
    pub half_extents: glam::Vec3,
    /// Rotation quaternion from local to world
    pub rotation: glam::Quat,
}

impl Obb {
    pub fn contains(&self, p: glam::Vec3) -> bool {
        // Transform point into OBB local space
        let inv = self.rotation.conjugate();
        let local = inv * (p - self.center);
        local.x.abs() <= self.half_extents.x &&
        local.y.abs() <= self.half_extents.y &&
        local.z.abs() <= self.half_extents.z
    }
}

/// Sphere volume
#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub center: glam::Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn contains(&self, p: glam::Vec3) -> bool {
        (p - self.center).length_squared() <= self.radius * self.radius
    }
}

/// General shape type for space/portal volumes
#[derive(Debug, Clone)]
pub enum Shape3D {
    Aabb(Aabb),
    Obb(Obb),
    Sphere(Sphere),
    /// Any-of union of child shapes
    Union(Vec<Shape3D>),
}

impl Shape3D {
    pub fn contains(&self, p: glam::Vec3) -> bool {
        match self {
            Shape3D::Aabb(a) => a.contains(p),
            Shape3D::Obb(o) => o.contains(p),
            Shape3D::Sphere(s) => s.contains(p),
            Shape3D::Union(children) => children.iter().any(|c| c.contains(p)),
        }
    }

    /// Contains test with a small margin for hysteresis. Positive margin expands the shape slightly.
    pub fn contains_with_margin(&self, p: glam::Vec3, margin: f32) -> bool {
        match self {
            Shape3D::Aabb(a) => {
                let m = glam::Vec3::splat(margin.max(0.0));
                let expanded = Aabb { min: a.min - m, max: a.max + m };
                expanded.contains(p)
            }
            Shape3D::Obb(o) => {
                let expanded = Obb { center: o.center, half_extents: o.half_extents + glam::Vec3::splat(margin.max(0.0)), rotation: o.rotation };
                expanded.contains(p)
            }
            Shape3D::Sphere(s) => {
                let e = Sphere { center: s.center, radius: (s.radius + margin.max(0.0)).max(0.0) };
                e.contains(p)
            }
            Shape3D::Union(children) => children.iter().any(|c| c.contains_with_margin(p, margin)),
        }
    }

    /// Segment intersection (rough, conservative): true if the movement segment intersects the shape.
    pub fn segment_intersects(&self, p0: glam::Vec3, p1: glam::Vec3) -> bool {
        match self {
            Shape3D::Aabb(a) => segment_intersects_aabb(p0, p1, *a),
            Shape3D::Obb(o) => {
                // transform segment into OBB local space then test against AABB of half_extents
                let inv = o.rotation.conjugate();
                let q0 = inv * (p0 - o.center);
                let q1 = inv * (p1 - o.center);
                segment_intersects_aabb(q0, q1, Aabb { min: -o.half_extents, max: o.half_extents })
            }
            Shape3D::Sphere(s) => segment_intersects_sphere(p0, p1, *s),
            Shape3D::Union(children) => children.iter().any(|c| c.segment_intersects(p0, p1)),
        }
    }

    /// Approximate center of the shape volume
    pub fn center(&self) -> glam::Vec3 {
        match self {
            Shape3D::Aabb(a) => (a.min + a.max) * 0.5,
            Shape3D::Obb(o) => o.center,
            Shape3D::Sphere(s) => s.center,
            Shape3D::Union(children) => children.first().map(|c| c.center()).unwrap_or(glam::Vec3::ZERO),
        }
    }
}

fn segment_intersects_aabb(p0: glam::Vec3, p1: glam::Vec3, a: Aabb) -> bool {
    // Liang-Barsky style slab intersection
    let mut tmin = 0.0f32;
    let mut tmax = 1.0f32;
    let d = p1 - p0;
    for i in 0..3 {
        let (p0i, di, min_i, max_i) = match i {
            0 => (p0.x, d.x, a.min.x, a.max.x),
            1 => (p0.y, d.y, a.min.y, a.max.y),
            _ => (p0.z, d.z, a.min.z, a.max.z),
        };
        if di.abs() < 1e-6 {
            if p0i < min_i || p0i > max_i { return false; }
        } else {
            let ood = 1.0 / di;
            let mut t1 = (min_i - p0i) * ood;
            let mut t2 = (max_i - p0i) * ood;
            if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax { return false; }
        }
    }
    true
}

fn segment_intersects_sphere(p0: glam::Vec3, p1: glam::Vec3, s: Sphere) -> bool {
    let m = p0 - s.center;
    let d = p1 - p0;
    let b = glam::Vec3::dot(m, d);
    let c = glam::Vec3::dot(m, m) - s.radius * s.radius;
    if c > 0.0 && b > 0.0 { return false; }
    let discr = b*b - c * glam::Vec3::dot(d, d);
    if discr < 0.0 { return false; }
    true
}

/// Medium types inside spaces (affects audio/physics later)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediumType { Air, Water }

/// A space volume (world/city/room); attach to an entity to mark a navigable area.
#[derive(Component, Debug, Clone)]
pub struct SpaceComponent {
    pub kind: SpaceKind,
    pub name: String,
    pub bounds: Shape3D,
    /// Whether the space has a ceiling (if false, vertical exit may be allowed)
    pub has_ceiling: bool,
    /// Medium inside this space (air or water)
    pub medium: MediumType,
}

impl SpaceComponent {
    pub fn center(&self) -> glam::Vec3 { self.bounds.center() }
}

/// Tracks which spaces an entity is currently inside (derived)
#[derive(Component, Debug, Clone, Default)]
pub struct InsideSpaces {
    pub spaces: HashSet<Entity>,
}

/// Simple navigation target and speed for entities that can move
#[derive(Component, Debug, Clone)]
pub struct NavigationState {
    pub target: Option<glam::Vec3>,
    pub speed: f32, // units per second
}

/// A sequence of waypoints to follow (world-space positions)
#[derive(Component, Debug, Clone, Default)]
pub struct NavigationPath {
    pub waypoints: Vec<glam::Vec3>,
    pub index: usize,
}

/// Door/window/portal connecting two spaces with a small volume the entity must cross
#[derive(Component, Debug, Clone)]
pub struct PortalComponent {
    pub from: Entity,
    pub to: Entity,
    pub shape: Shape3D,
    pub bidirectional: bool,
    pub is_open: bool,
    /// If non-zero, only entities whose traversal mask intersects this mask may pass
    pub allow_mask: u64,
    /// If non-zero, entity must possess all required abilities to pass
    pub required_abilities_mask: u64,
    /// Optional traversal cost hint (for pathfinding)
    pub cost: f32,
}

/// Tag registry providing stable bit indices for string tags
#[derive(Resource, Debug, Default, Clone)]
pub struct TagRegistry {
    pub map: std::collections::HashMap<String, u8>,
    pub next_bit: u8,
}

impl TagRegistry {
    pub fn bit_for(&mut self, tag: &str) -> u8 {
        if let Some(&b) = self.map.get(tag) { return b; }
        let b = self.next_bit.min(63);
        self.map.insert(tag.to_string(), b);
        if self.next_bit < 63 { self.next_bit += 1; }
        b
    }
    pub fn mask_for<'a, I: IntoIterator<Item=&'a str>>(&mut self, tags: I) -> u64 {
        let mut m = 0u64;
        for t in tags { m |= 1u64 << self.bit_for(t); }
        m
    }
}

/// Bitmask tags describing traversal affiliations (e.g., player/npc)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TraversalMask {
    pub mask: u64,
}

/// Optional tag mask describing properties of a space (for querying/filtering)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SpaceTags {
    pub mask: u64,
}

/// Generic abilities on actors (bitmask); complements specific flags like CanClimb/CanDive
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Abilities {
    pub mask: u64,
}

/// Ability registry providing stable bit indices for string ability names
#[derive(Resource, Debug, Default, Clone)]
pub struct AbilityRegistry {
    pub map: std::collections::HashMap<String, u8>,
    pub next_bit: u8,
}

impl AbilityRegistry {
    pub fn bit_for(&mut self, name: &str) -> u8 {
        if let Some(&b) = self.map.get(name) { return b; }
        let b = self.next_bit.min(63);
        self.map.insert(name.to_string(), b);
        if self.next_bit < 63 { self.next_bit += 1; }
        b
    }
    pub fn mask_for<'a, I: IntoIterator<Item=&'a str>>(&mut self, names: I) -> u64 {
        let mut m = 0u64;
        for n in names { m |= 1u64 << self.bit_for(n); }
        m
    }
}

/// Ability flag: can climb walls to exit spaces without using portals
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CanClimb;

/// Ability flag: can dive/swim into water spaces
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CanDive;

/// Previous frame position, used for detecting portal crossings
#[derive(Component, Debug, Clone, Copy)]
pub struct PreviousPosition(pub glam::Vec3);

/// Precomputed space graph indices to speed up portal lookups
#[derive(Resource, Debug, Default, Clone)]
pub struct SpaceGraph {
    /// Map space entity -> portal entity IDs that originate from that space
    pub portals_from: std::collections::HashMap<Entity, Vec<Entity>>,
    /// Map space entity -> portal entity IDs that lead into that space (reverse edges)
    pub portals_to: std::collections::HashMap<Entity, Vec<Entity>>,
}

/// Enable/Configure navmesh-based guidance cues for an entity
#[derive(Component, Debug, Clone, Copy)]
pub struct NavmeshGuidance {
    pub boundary_warn_distance: f32, // meters
    pub turn_cue_angle_deg: f32,     // degrees
}
