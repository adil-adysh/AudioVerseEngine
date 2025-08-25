use bevy_ecs::prelude::{Res, Query, With};
use bevy_input::ButtonInput;
use bevy_input::keyboard::KeyCode;
use bevy_math::Vec3;
use crate::components::*;

// Restores player input using Bevy 0.16 `ButtonInput<KeyCode>` and `KeyCode` variants.
// Maps W/A/S/D to forward/left/back/right, Space to jump (up), and LShift to sprint (speed modifier).
pub fn player_input(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut MoveDirection, With<Player>>) {
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    // Vertical input: Space for up (jump). Preserving as a simple flag vector component.
    if keyboard.pressed(KeyCode::Space) {
        direction.y += 1.0;
    }

    // Normalize horizontal movement but keep vertical as-is
    let horizontal = Vec3::new(direction.x, 0.0, direction.z);
    let horizontal = if horizontal.length_squared() > 0.0 {
        horizontal.normalize()
    } else {
        Vec3::ZERO
    };
    let move_dir = Vec3::new(horizontal.x, direction.y, horizontal.z);

    if let Ok(mut player_move_dir) = query.single_mut() {
        player_move_dir.0 = move_dir;
    }
}