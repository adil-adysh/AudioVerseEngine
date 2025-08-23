use bevy_ecs::prelude::*;

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
