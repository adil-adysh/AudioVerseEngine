use bevy_app::{App, FixedUpdate, PostUpdate, Plugin, Update};
use bevy_rapier3d::plugin::RapierPhysicsPlugin;
use bevy_rapier3d::prelude::NoUserData;
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use std::collections::HashMap;

// Declare all of our custom modules.
mod components;
#[cfg(feature = "world-loader")]
mod world_loader;

// Heavy modules are feature-gated to allow a minimal compile while we
// iteratively fix their implementations. Enable `full_engine` to build
// with the full set of systems.
#[cfg(feature = "full_engine")]
mod physics;
#[cfg(feature = "full_engine")]
mod portal;
#[cfg(feature = "full_engine")]
mod audio;
#[cfg(feature = "full_engine")]
mod systems;

use components::*;
#[cfg(feature = "world-loader")]
use world_loader::*;

// If the full modules are not compiled, provide lightweight stubs so the
// plugin can still be built. These are replaced when `full_engine` is set.
#[cfg(not(feature = "full_engine"))]
mod stubs {
    pub fn player_input() {}
    pub fn update_listener_position_system() {}
    pub fn environmental_effects_system() {}
    pub fn audio_occlusion_system() {}
    pub fn doppler_effect_system() {}
    pub fn portal_trigger_system() {}
    pub fn handle_teleport_system() {}
    pub fn kinematic_controller_update_system() {}
    pub fn update_player_state_system() {}
}
#[cfg(not(feature = "full_engine"))]
use stubs::*;

// This is the main plugin for our game logic.
// All of our systems and resources are added here.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // Add our physics engine plugin (we're avoiding pulling full Bevy here).
        app.add_plugins((
            RapierPhysicsPlugin::<NoUserData>::default(),
            // Add this plugin to visualize the physics colliders for debugging.
            RapierDebugRenderPlugin::default(),
        ));
        // Add our custom resource for managing audio assets. Use a
        // lightweight placeholder when full_engine is not enabled.
        #[cfg(not(feature = "full_engine"))]
        {
            use bevy_ecs::prelude::Resource;

            #[derive(Default, Resource)]
            struct AudioAssetsPlaceholder {
                sounds: HashMap<String, ()>,
            }
            app.insert_resource(AudioAssetsPlaceholder { sounds: HashMap::new() });
        }
        #[cfg(feature = "full_engine")]
        {
            app.insert_resource(AudioAssets {
                sounds: HashMap::new(),
            });
        }
        // Add all of our custom systems to the game's update loop.
        app.add_systems(Update, (
            // Input System
            player_input,

            // Audio Systems
            update_listener_position_system,
            environmental_effects_system,
            audio_occlusion_system,
            doppler_effect_system,

            // Portal Systems
            portal_trigger_system,
            handle_teleport_system,

            // World and Entity Systems (optional)
    ));

        // Register world loader systems only when the feature is enabled.
        #[cfg(feature = "world-loader")] {
            app.add_systems(Update, (
                load_world_system,
                spawn_entities_system,
            ));
        }

        // Add our physics systems to the correct stages to ensure they run
        // in sync with the physics engine.
        app.add_systems(FixedUpdate, kinematic_controller_update_system);
        app.add_systems(PostUpdate, update_player_state_system);
    }
}