use bevy::prelude::*;
// no explicit Camera bundle import to keep the app minimal and dependency-free

// Pull in our engine core plugin
use engine_core::GamePlugin;
use engine_core::{AudioAssets, Player, MoveDirection, SoundEmitter};

fn main() {
    // Create a minimal Bevy app with default plugins sufficient to exercise systems
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((GamePlugin,))
    .add_systems(Startup, spawn_test)
    // Bevy 0.16 uses the `add_systems` API with stages.
    .add_systems(Update, movement_input_system)
        .run();
}

fn spawn_test(
    mut audio_assets: ResMut<AudioAssets>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    // Audio assets are populated by the engine `WorldPlugin` on startup.
    // If needed, application-level assets can be loaded here and inserted
    // into `AudioAssets`. We rely on the engine to have already added
    // "preview" from `assets/Preview.ogg`.

    // Spawn a player entity with a movement component and a sound emitter.
    commands.spawn((Player, MoveDirection(bevy::math::Vec3::ZERO), SoundEmitter { sound_id: "preview".to_string(), volume: 1.0, velocity: bevy::math::Vec3::ZERO }));
}

// Tracks whether the player was moving in the previous frame.
#[derive(Resource, Default)]
struct LastMove(pub bool);

fn movement_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut MoveDirection, With<Player>>,
    audio_assets: Res<AudioAssets>,
    mut last: Local<LastMove>,
) {
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.z += 1.0;
    }
    // Arrow keys map to horizontal movement: Left/Right -> x, Up/Down -> z
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        direction.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if keyboard.pressed(KeyCode::Space) {
        direction.y += 1.0;
    }
    // PageUp/PageDown adjust vertical movement (y)
    if keyboard.pressed(KeyCode::PageUp) {
        direction.y += 1.0; // climb
    }
    if keyboard.pressed(KeyCode::PageDown) {
        direction.y -= 1.0; // descend
    }

    let horizontal = Vec3::new(direction.x, 0.0, direction.z);
    let horizontal = if horizontal.length_squared() > 0.0 {
        horizontal.normalize()
    } else {
        Vec3::ZERO
    };
    let move_dir = Vec3::new(horizontal.x, direction.y, horizontal.z);

    if let Ok(mut md) = query.single_mut() {
        let was_moving = last.0;
        let is_moving = move_dir.length_squared() > 0.0;
        if is_moving && !was_moving {
            info!("movement started: {:?}", move_dir);
            if audio_assets.sounds.get("preview").is_some() {
                info!("preview sound loaded and available");
            } else {
                warn!("preview sound not loaded");
            }
        } else if !is_moving && was_moving {
            info!("movement stopped");
        } else if is_moving {
            debug!("moving: {:?}", move_dir);
        }
        md.0 = move_dir;
        last.0 = is_moving;
    }
}
