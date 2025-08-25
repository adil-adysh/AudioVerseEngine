use bevy_ecs::prelude::{World, Entity, Bundle};
use bevy_transform::components::Transform;
use bevy_math::Vec3;

use crate::components::*;
use bevy_tnua::controller::TnuaController;

/// Spawn a player with the minimal set of components required by the
/// engine's tnua-only physics systems.
pub fn spawn_player(world: &mut World, translation: Vec3) -> Entity {
    let bundle = PlayerBundle {
        player: Player,
        move_direction: MoveDirection(Vec3::ZERO),
        movement_speed: MovementSpeed(5.0),
        has_collider: HasCollider,
        transform: Transform::from_translation(translation),
        velocity: Velocity(Vec3::ZERO),
        tnua: TnuaController::default(),
    };
    let cmd = world.spawn(bundle);
    cmd.id()
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub move_direction: MoveDirection,
    pub movement_speed: MovementSpeed,
    pub has_collider: HasCollider,
    pub transform: Transform,
    pub velocity: Velocity,
    pub tnua: TnuaController,
}
