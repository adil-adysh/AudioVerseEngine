//! Minimal engine-core facade using bevy_ecs as the ECS backend.
//! This is a lightweight start matching the engine public API flow.

pub mod components;
pub mod engine;
pub mod events;
pub mod pathfinding;
pub mod physics;
pub mod plugin;
pub mod systems;
pub mod transform;
#[cfg(feature = "world-loader")]
pub mod world_loader;

// Optional Bevy ecosystem integrations (navmesh + raycast)
mod bevy_extras;

pub use engine::Engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creation() {
        let _ = Engine::new();
    }
}
