use bevy_ecs::prelude::{Query, Res, ResMut, With};
use bevy_time::Time;
use bevy_transform::components::Transform;
use bevy_math::Vec3;

use bevy_tnua::controller::TnuaController;
use bevy_tnua::builtins::TnuaBuiltinWalk;
use bevy_ecs::prelude::Resource;

use crate::components::*;

/// Small resource to remember the player's previous position so we can compute
/// an empirical velocity from the actual Transform delta after physics.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PrevPlayerPos(pub Option<Vec3>);

/// Feed the Tnua controller from our simple MovementSpeed + MoveDirection
/// components. This crate requires that the game always uses Tnua for
/// character movement (no fallbacks), so we expect the entity to have a
/// `TnuaController` component.
pub fn kinematic_controller_update_system(
    mut tnua_query: Query<(&MovementSpeed, &MoveDirection, &mut TnuaController), With<Player>>,
    _time: Res<Time>,
) {
    if let Ok((speed, direction, mut controller)) = tnua_query.single_mut() {
    let mut basis = TnuaBuiltinWalk::default();
    basis.desired_velocity = direction.0 * speed.0;
    basis.desired_forward = None;
    basis.float_height = 2.0;
    controller.basis(basis);
    }
}

/// After physics, compute the player's velocity by comparing the current
/// Transform with the previous frame's position. Update the `Velocity` component
/// and orient the Transform to face the movement direction when appropriate.
pub fn update_player_state_system(
    mut query: Query<(&mut Velocity, &mut Transform), With<Player>>,
    mut prev_pos: ResMut<PrevPlayerPos>,
    time: Res<Time>,
) {
    if let Ok((mut velocity, mut transform)) = query.single_mut() {
        let dt = time.delta_secs();
        let current = transform.translation;

        if let Some(last) = prev_pos.0 {
            if dt > 0.0 {
                velocity.0 = (current - last) / dt;
            }
        } else {
            // First frame: assume zero velocity.
            velocity.0 = Vec3::ZERO;
        }

        // Update facing direction based on current velocity.
        if velocity.0.length_squared() > 1e-6 {
            let target = current + velocity.0.normalize();
            transform.look_at(target, Vec3::Y);
        }

        prev_pos.0 = Some(current);
    }
}