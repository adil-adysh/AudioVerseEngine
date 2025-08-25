use bevy_app::{App, Plugin, Update, FixedUpdate, PostUpdate};
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
mod physics;
mod portal;
mod audio;
mod systems;

use components::*;
#[cfg(feature = "world-loader")]
use world_loader::*;
use physics::*;
use portal::*;
use audio::*;
use systems::*;

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
    // Install the audio plugin which registers audio resources and systems.
    app.add_plugins((AudioPlugin,));

    // Add our custom systems to the game's update loop individually to
    // keep the API surface simple during migration.
    app.add_systems(Update, player_input);
    app.add_systems(Update, doppler_effect_system);
    app.add_systems(Update, portal_trigger_system);
    app.add_systems(Update, handle_teleport_system);

    // Add our physics systems to the correct stages to ensure they run
    // in sync with the physics engine. These systems are enabled by
    // default per project policy (no feature gates).
    app.add_systems(FixedUpdate, kinematic_controller_update_system);
    app.add_systems(PostUpdate, update_player_state_system);
    }
}