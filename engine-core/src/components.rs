use bevy_ecs::prelude::{Entity, Component};
use bevy_time::Timer;
use bevy_math::Vec3;
#[cfg(feature = "world-loader")]
use serde::Deserialize;

// Player and Movement
#[derive(Component, Debug, Clone, Default)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct Player;
#[derive(Component, Debug, Clone, Copy)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct MovementSpeed(pub f32);
#[derive(Component, Debug, Clone, Copy)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct MoveDirection(pub Vec3);
#[derive(Component, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct HasCollider; // Marker to easily find entities with a collider

// Physics
#[derive(Component, Debug, Clone, Copy)] // Removed Deserialize
pub struct Velocity(pub Vec3);
#[derive(Component, Debug, Clone, Copy)] // Removed Deserialize
pub struct ExternalForce(pub Vec3);

// Audio
#[derive(Component, Debug, Clone, Default)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct SoundEmitter {
    pub sound_id: String,
    pub volume: f32,
    pub velocity: Vec3, // Added for Doppler effect
}
#[derive(Component, Debug, Clone, Default)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct SoundMaterial {
    pub occlusion_strength: f32, // 0.0 for no obstruction, 1.0 for full
    pub reflectivity: f32, // Added for dynamic reflections
}
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CurrentMedium {
    pub medium: MediumType,
}
#[derive(Component, Debug, Clone, Default)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct CurrentAcousticSpace {
    pub entity: Option<Entity>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub enum MediumType {
    #[cfg_attr(feature = "world-loader", serde(rename = "air"))]
    Air,
    #[cfg_attr(feature = "world-loader", serde(rename = "water"))]
    Water,
}

impl Default for MediumType {
    fn default() -> Self {
        MediumType::Air
    }
}

// Environmental Volumes
#[derive(Component, Debug, Clone)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct AcousticVolume {
    pub shape: VolumeShape,
    pub reverb_strength: f32,
}
#[derive(Component, Debug, Clone)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct MediumVolume {
    pub shape: VolumeShape,
    pub medium_type: MediumType,
}
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub enum VolumeShape {
    #[cfg_attr(feature = "world-loader", serde(rename = "aabb"))]
    Aabb(Aabb),
    #[cfg_attr(feature = "world-loader", serde(rename = "sphere"))]
    Sphere(Sphere),
}
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Aabb {
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
}

impl Sphere {
    pub fn contains(&self, point: Vec3) -> bool {
        self.center.distance_squared(point) <= self.radius * self.radius
    }
}

impl VolumeShape {
    pub fn contains(&self, point: Vec3) -> bool {
        match self {
            VolumeShape::Aabb(aabb) => aabb.contains(point),
            VolumeShape::Sphere(sphere) => sphere.contains(point),
        }
    }
}

// Portal System
#[derive(Component, Debug, Clone)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct Portal {
    pub destination: Vec3,
    pub volume_shape: VolumeShape,
}
#[derive(Component, Debug, Clone)] // Removed Default to prevent timer bug
pub struct Teleporting {
    pub destination: Vec3,
    pub timer: Timer,
}

// Data-Driven World Loading
#[derive(Component, Debug, Clone, Default)]
#[cfg_attr(feature = "world-loader", derive(Deserialize))]
pub struct WorldParent(pub String);