use bevy::prelude::{Commands, Query, Entity, Transform, Res, With, Timer, TimerMode, Time};

use crate::components::*;

/// This system detects when a player enters a portal's volume.
/// Instead of teleporting instantly, it adds the `Teleporting` component,
/// which initiates a timed transition.
pub fn portal_trigger_system(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, With<HasCollider>)>,
    portal_query: Query<(&Portal, &Transform)>,
    teleporting_query: Query<&Teleporting>,
) {
    // Check if a player entity exists and is not already in a teleporting state.
    if let Ok((player_entity, player_transform)) = player_query.get_single() {
        if teleporting_query.get_single().is_ok() {
            return;
        }
        
        // Iterate through all portals to check for a trigger.
        for (portal, _) in portal_query.iter() {
            let player_position = player_transform.translation;
            let portal_shape = portal.volume_shape;

            if portal_shape.contains(player_position) {
                // Add the Teleporting component to the player,
                // initiating a timed transition.
                commands.entity(player_entity).insert(Teleporting {
                    destination: portal.destination,
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                });
                info!("Player entered a portal, initiating teleportation...");
                return; // Only trigger one portal at a time.
            }
        }
    }
}

/// This system handles the actual teleportation once the `Teleporting`
/// component's timer has finished.
pub fn handle_teleport_system(
    mut commands: Commands,
    mut player_query: Query<(Entity, &mut Transform, &mut Teleporting)>,
    time: Res<Time>,
) {
    if let Ok((player_entity, mut player_transform, mut teleporting)) = player_query.get_single_mut() {
        // Tick the timer.
        teleporting.timer.tick(time.delta());

        // Check if the timer has finished.
        if teleporting.timer.finished() {
            // Perform the teleport.
            player_transform.translation = teleporting.destination;
            info!("Player teleported to new destination!");

            // Remove the Teleporting component to stop the transition.
            commands.entity(player_entity).remove::<Teleporting>();
        }
    }
}