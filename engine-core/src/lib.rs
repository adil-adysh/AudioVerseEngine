//! Minimal engine-core facade using bevy_ecs as the ECS backend.
//! This is a lightweight start matching the engine public API flow.

pub mod components;
pub mod events;
pub mod engine;

pub use engine::Engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creation() {
        let _ = Engine::new();
    }
}
