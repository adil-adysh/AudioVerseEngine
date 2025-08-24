use bevy::prelude::{Res, Query, With, KeyCode, Input, Vec3};
use crate::components::*;

// System to handle keyboard input for player movement on all three axes.
pub fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut MoveDirection, With<Player>>,
) {
    let mut move_direction = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::W) {
        move_direction.z -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        move_direction.z += 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) {
        move_direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        move_direction.x += 1.0;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        move_direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::LShift) {
        move_direction.y -= 1.0;
    }
    
    if move_direction != Vec3::ZERO {
        move_direction = move_direction.normalize();
    }
    
    if let Ok(mut player_move_dir) = query.get_single_mut() {
        player_move_dir.0 = move_direction;
    }
}