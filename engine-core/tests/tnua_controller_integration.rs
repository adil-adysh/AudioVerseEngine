use bevy_app::App;
use bevy::prelude::{MinimalPlugins, Update, FixedUpdate, PostUpdate, Vec3};

use engine_core::{spawn_player, player_input, kinematic_controller_update_system, update_player_state_system, MoveDirection, Velocity, Player};

#[test]
fn tnua_controller_moves_player_on_input() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Insert PrevPlayerPos resource so the post-update system can access it.
    app.world_mut().insert_resource(engine_core::PrevPlayerPos::default());

    // Register systems in the proper stages.
    app.add_systems(Update, player_input);
    app.add_systems(FixedUpdate, kinematic_controller_update_system);
    app.add_systems(PostUpdate, update_player_state_system);

    // Spawn player via helper at origin and remember the entity id.
    let player = spawn_player(app.world_mut(), Vec3::ZERO);

    // Simulate pressing W (forward).
    let mut keyboard = bevy::input::ButtonInput::<bevy::input::keyboard::KeyCode>::default();
    keyboard.press(bevy::input::keyboard::KeyCode::KeyW);
    app.world_mut().insert_resource(keyboard);

    // Run one frame cycle: Update, FixedUpdate, PostUpdate
    app.update();

    // After update, ensure player has non-zero velocity on Z axis (forward)
    let v = app.world().get::<Velocity>(player).expect("Player entity should have Velocity");
    assert!(v.0.z.abs() > 1e-6, "Player should have non-zero forward velocity after input and tnua controller update");
}
