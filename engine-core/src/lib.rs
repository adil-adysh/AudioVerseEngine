use bevy_app::{App, Plugin, Update, FixedUpdate, PostUpdate};
use bevy_rapier3d::plugin::RapierPhysicsPlugin;
use bevy_rapier3d::prelude::NoUserData;
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use std::collections::HashMap;
use bevy_tnua::controller::TnuaControllerPlugin;
mod spawn;

// Declare all of our custom modules.
mod components;
// World loader is feature-gated and under active development.
// Temporarily omit the module from the default build to avoid
// pulling serde/glam/type-layout dependencies during workspace
// compilation. Re-enable by restoring the `mod world_loader;` line
// and enabling the `world-loader` feature.
// #[cfg(feature = "world-loader")]
// mod world_loader;

// Heavy modules are feature-gated to allow a minimal compile while we
// iteratively fix their implementations. Enable `full_engine` to build
// with the full set of systems.
mod physics;
mod portal;
pub mod audio;
mod world;
mod systems;

pub use components::*;
pub use physics::*;
pub use portal::*;
pub use audio::*;
pub use world::*;
pub use systems::*;
pub use spawn::spawn_player;
pub use spawn::PlayerBundle;

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
    // Add Tnua controller plugin (character controller helpers).
    app.add_plugins(TnuaControllerPlugin::default());
    // Insert helper resource used by our player state update system.
    app.insert_resource(crate::physics::PrevPlayerPos::default());
    // Install the audio plugin which registers audio resources and systems.
    app.add_plugins((AudioPlugin, WorldPlugin));

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