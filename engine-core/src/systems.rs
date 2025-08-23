use bevy_ecs::prelude::*;

use crate::components::{
    AudioListenerComponent,
    WorldTransformComponent,
    TransformComponent,
    NavigationState,
    SpaceComponent,
    InsideSpaces,
    PortalComponent,
    TraversalTags,
    CanClimb,
    CanDive,
    PreviousPosition,
};
use crate::events::{ListenerTransformEvent, NavigateToEvent, EnterSpaceEvent, ExitSpaceEvent};

/// Simple time step resource for variable-loop systems
#[derive(Resource, Debug, Clone, Copy)]
pub struct TimeStep(pub f32);

/// Finds the active listener and publishes its world transform as an event.
pub fn audio_listener_system(
    mut writer: ResMut<Events<ListenerTransformEvent>>,
    query: Query<(Entity, &AudioListenerComponent, Option<&WorldTransformComponent>)>,
) {
    for (e, _listener, wt) in query.iter() {
        if let Some(wt) = wt {
            writer.send(ListenerTransformEvent { entity: e, matrix: wt.matrix });
        }
        break; // single listener for now
    }
}

/// Minimal audio system placeholder that consumes play/pause/stop events.
#[derive(Debug)]
pub struct PlaySoundEvent { pub entity: u32 }pub fn audio_system() {}

/// Moves entities toward their navigation targets (simple Euler step).
pub fn navigation_system(
    mut nav_events: ResMut<Events<NavigateToEvent>>,
    mut query: Query<(Entity, &mut TransformComponent, Option<&mut NavigationState>)>,
) {
    // Drain navigate-to commands by applying target/speed to entities.
    let mut drained = Vec::new();
    nav_events.drain().for_each(|e| drained.push(e));
    for e in drained {
        if let Ok((_entity, _t, nav_opt)) = query.get_mut(e.entity) {
            if let Some(mut nav) = nav_opt {
                nav.target = Some(e.target);
                nav.speed = e.speed.max(0.0);
            }
        }
    }
}

/// Applies movement toward target each frame using TimeStep.
pub fn navigation_step_system(
    time: Option<Res<TimeStep>>,
    mut query: Query<(&mut TransformComponent, &mut NavigationState, Option<&mut PreviousPosition>)>,
) {
    let dt = time.map(|t| t.0).unwrap_or(0.0);
    if dt <= 0.0 { return; }
    for (mut t, mut nav, prev_opt) in query.iter_mut() {
        // cache previous position
        if let Some(mut prev) = prev_opt { prev.0 = t.position; }
        if let Some(target) = nav.target {
            let to = target - t.position;
            let dist = to.length();
            if dist > 0.0001 {
                let max_step = nav.speed.max(0.0) * dt;
                let step = if dist <= max_step { dist } else { max_step };
                let dir = to / dist;
                t.position += dir * step;
                if step >= dist { nav.target = None; }
            } else {
                nav.target = None;
            }
        }
    }
}

/// Detects when entities enter/exit spaces based on AABB contains() test.
pub fn space_membership_system(
    mut enter_events: ResMut<Events<EnterSpaceEvent>>,
    mut exit_events: ResMut<Events<ExitSpaceEvent>>,
    mut movers: Query<(Entity, &TransformComponent, Option<&mut InsideSpaces>, Option<&TraversalTags>, Option<&CanClimb>, Option<&CanDive>, Option<&PreviousPosition>)>,
    spaces: Query<(Entity, &SpaceComponent)>,
    portals: Query<&PortalComponent>,
) {
    // Build a list of all spaces for iteration.
    let space_list: Vec<(Entity, SpaceComponent)> = spaces
        .iter()
        .map(|(e, sc)| (e, sc.clone()))
        .collect();

    for (ent, t, inside_opt, tags_opt, can_climb, can_dive, prev_pos) in movers.iter_mut() {
    let current: std::collections::HashSet<Entity> = inside_opt
            .as_ref()
            .map(|i| i.spaces.clone())
            .unwrap_or_default();

        // Determine new set.
        let mut new_set = std::collections::HashSet::new();
        for (space_ent, sc) in space_list.iter() {
            if sc.bounds.contains(t.position) {
                new_set.insert(*space_ent);
            }
        }

        // Emit Enter events for newly entered spaces.
        for space_ent in new_set.difference(&current) {
            enter_events.send(EnterSpaceEvent { entity: ent, space: *space_ent });
        }
        // Emit Exit events for spaces no longer inside.
        let mut to_retain: Vec<Entity> = Vec::new();
        for space_ent in current.difference(&new_set) {
            // Check if exit is permitted: via portal or allowed vertical/hard traversal
            let mut permitted = false;
            // If there are no portals defined from this space at all, permit by default
            // except when the space has no ceiling, in which case require CanClimb.
            let mut has_any_portal = false;
            for p in portals.iter() {
                if p.from == *space_ent || (p.bidirectional && p.to == *space_ent) { has_any_portal = true; break; }
            }
            if !has_any_portal {
                if let Some((_e, sc)) = space_list.iter().find(|(e, _)| e == space_ent) {
                    permitted = if sc.has_ceiling { true } else { can_climb.is_some() };
                } else {
                    permitted = true; // conservative default
                }
            }
            // If we have previous position, test portal crossings between prev and curr
            if let Some(prev) = prev_pos {
                // Search portals that go from this space to any other
                for portal in portals.iter() {
                    if portal.from == *space_ent && portal.shape.segment_intersects(prev.0, t.position) {
                        if portal.is_open && match &portal.allow_tags {
                            None => true,
                            Some(needed) => tags_opt.map_or(false, |tags| {
                                tags.tags.iter().any(|tag| needed.contains(tag))
                            }),
                        } {
                            permitted = true;
                            break;
                        }
                    }
                    if portal.bidirectional && portal.to == *space_ent && portal.shape.segment_intersects(prev.0, t.position) {
                        if portal.is_open && match &portal.allow_tags {
                            None => true,
                            Some(needed) => tags_opt.map_or(false, |tags| {
                                tags.tags.iter().any(|tag| needed.contains(tag))
                            }),
                        } {
                            permitted = true;
                            break;
                        }
                    }
                }
            }

            // If the space has no ceiling and the entity can climb, allow exit
            if !permitted {
                if let Some((_s_ent, sc)) = space_list.iter().find(|(e, _)| e == space_ent) {
                    if !sc.has_ceiling && can_climb.is_some() {
                        permitted = true;
                    }
                    // If moving into water and entity can dive, allow
                    if sc.medium == crate::components::MediumType::Water && can_dive.is_some() {
                        permitted = true;
                    }
                }
            }

            if permitted { exit_events.send(ExitSpaceEvent { entity: ent, space: *space_ent }); }
            else { to_retain.push(*space_ent); }
        }
        // Retain membership if exit not permitted (apply after iteration to avoid aliasing)
        for s in to_retain { new_set.insert(s); }

        // Write back membership component.
    if let Some(mut inside) = inside_opt.and_then(|o| o.into()) {
            inside.spaces = new_set;
        } else {
            // Insert via commands is not available here; rely on caller to ensure component exists
            // or we can choose to skip insertion to avoid Commands dependency in this module.
        }
    }
}
