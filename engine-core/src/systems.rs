use bevy_ecs::prelude::*;

use crate::components::{
    AudioListenerComponent,
    WorldTransformComponent,
    TransformComponent,
    NavigationState,
    SpaceComponent,
    InsideSpaces,
    PortalComponent,
    TraversalMask,
    CanClimb,
    CanDive,
    PreviousPosition,
    SpaceGraph,
};
use crate::events::{ListenerTransformEvent, NavigateToEvent, EnterSpaceEvent, ExitSpaceEvent};
use crate::events::AcousticsEvent;
use crate::components::MediumType;
use crate::navmesh as nm;

/// Simple time step resource for variable-loop systems
#[derive(Resource, Debug, Clone, Copy)]
pub struct TimeStep(pub f32);

/// Finds the active listener and publishes its world transform as an event.
pub fn audio_listener_system(
    mut writer: ResMut<Events<ListenerTransformEvent>>,
    query: Query<(Entity, &AudioListenerComponent, Option<&WorldTransformComponent>, Option<&TransformComponent>)>,
) {
    if let Some((e, _l, wt_opt, t_opt)) = query.iter().next() {
        let mat = if let Some(wt) = wt_opt {
            wt.matrix
        } else if let Some(t) = t_opt {
            glam::Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position)
        } else {
            return;
        };
        writer.send(ListenerTransformEvent { entity: e, matrix: mat });
    }
}

/// Minimal audio system placeholder that consumes play/pause/stop events.
pub fn audio_system(
    mut play: ResMut<Events<crate::events::PlaySoundEvent>>,
    mut stop: ResMut<Events<crate::events::StopSoundEvent>>,
    mut pause: ResMut<Events<crate::events::PauseSoundEvent>>,
    mut vol: ResMut<Events<crate::events::SetVolumeEvent>>,
    external_enabled: Option<Res<ExternalAudioSystemEnabled>>,
) {
    // If an external audio integration is enabled, do not drain events here.
    if let Some(flag) = external_enabled { if flag.0 { return; } }
    // Drain to avoid unbounded growth; real system would forward to audio backend
    play.drain().for_each(|_e| {});
    stop.drain().for_each(|_e| {});
    pause.drain().for_each(|_e| {});
    vol.drain().for_each(|_e| {});
}

/// Resource flag to signal that an external audio integration will consume audio events.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ExternalAudioSystemEnabled(pub bool);

/// Moves entities toward their navigation targets (simple Euler step).
pub fn navigation_system(
    mut nav_events: ResMut<Events<NavigateToEvent>>,
    mut query: Query<(Entity, &mut TransformComponent, Option<&mut NavigationState>)>,
) {
    // Drain navigate-to commands by applying target/speed to entities.
    let mut drained = Vec::new();
    nav_events.drain().for_each(|e| drained.push(e));
    for e in drained {
        if let Ok((_entity, _t, Some(mut nav))) = query.get_mut(e.entity) {
            nav.target = Some(e.target);
            nav.speed = e.speed.max(0.0);
        }
    }
}

/// Applies movement toward target each frame using TimeStep.
pub fn navigation_step_system(
    time: Option<Res<TimeStep>>,
    mut query: Query<(&mut TransformComponent, &mut NavigationState, Option<&mut PreviousPosition>, Option<&mut crate::components::NavigationPath>)>,
) {
    let dt = time.map(|t| t.0).unwrap_or(0.0);
    if dt <= 0.0 { return; }
    for (mut t, mut nav, prev_opt, mut path_opt) in query.iter_mut() {
        // cache previous position
        if let Some(mut prev) = prev_opt { prev.0 = t.position; }
        // if we have a path, set current target to next waypoint
    if let Some(ref mut path) = path_opt {
            if path.index < path.waypoints.len() {
                nav.target = Some(path.waypoints[path.index]);
            }
        }
        if let Some(target) = nav.target {
            let to = target - t.position;
            let dist = to.length();
            if dist > 0.0001 {
                let max_step = nav.speed.max(0.0) * dt;
                let step = if dist <= max_step { dist } else { max_step };
                let dir = to / dist;
                t.position += dir * step;
                if step >= dist {
                    // reached target; advance path if any
                    nav.target = None;
                    if let Some(ref mut path) = path_opt {
                        if path.index < path.waypoints.len() { path.index += 1; }
                        if path.index < path.waypoints.len() {
                            nav.target = Some(path.waypoints[path.index]);
                        }
                    }
                }
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
    mut acoustics_events: ResMut<Events<AcousticsEvent>>,
    mut movers: Query<(Entity, &TransformComponent, Option<&mut InsideSpaces>, Option<&TraversalMask>, Option<&CanClimb>, Option<&CanDive>, Option<&PreviousPosition>, Option<&crate::components::Abilities>)>,
    spaces: Query<(Entity, &SpaceComponent)>,
    portals: Query<(Entity, &PortalComponent)>,
    graph: Option<Res<SpaceGraph>>,
) {
    // Build a list of all spaces for iteration.
    let space_list: Vec<(Entity, SpaceComponent)> = spaces
        .iter()
        .map(|(e, sc)| (e, sc.clone()))
        .collect();

    // small hysteresis margins to stabilize transitions
    let enter_margin = 0.05f32;

    for (ent, t, inside_opt, tags_opt, can_climb, can_dive, prev_pos, abilities_opt) in movers.iter_mut() {
        let current: std::collections::HashSet<Entity> = inside_opt
            .as_ref()
            .map(|i| i.spaces.clone())
            .unwrap_or_default();

        // Determine new set.
        let mut new_set = std::collections::HashSet::new();
        // Geometry-only candidates: spaces that contain current position ignoring medium gating
        let mut geom_candidates: Vec<Entity> = Vec::new();
        for (space_ent, sc) in space_list.iter() {
            if sc.bounds.contains_with_margin(t.position, enter_margin) {
                geom_candidates.push(*space_ent);
                // Gate entering water by CanDive
                if sc.medium == MediumType::Water && can_dive.is_none() {
                    // Cannot enter water without dive ability
                } else {
                    new_set.insert(*space_ent);
                }
            }
        }

    // (Enter events and medium change moved after portal/exit resolution)
        // Emit Exit events for spaces no longer inside.
    let mut to_retain: Vec<Entity> = Vec::new();
    let mut to_add: Vec<Entity> = Vec::new();
        for space_ent in current.difference(&new_set) {
            // Check if exit is permitted: via portal or allowed vertical/hard traversal
            let mut permitted = false;
            // If there are no portals defined from this space at all, permit by default
            // except when the space has no ceiling, in which case require CanClimb.
            let mut has_any_portal = false;
            match graph.as_ref() {
                Some(g) => {
                    if let Some(v) = g.portals_from.get(space_ent) { has_any_portal |= !v.is_empty(); }
                    if let Some(v) = g.portals_to.get(space_ent) { has_any_portal |= !v.is_empty(); }
                }
                None => {
                    for (_pe, p) in portals.iter() {
                        if p.from == *space_ent || (p.bidirectional && p.to == *space_ent) { has_any_portal = true; break; }
                    }
                }
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
                // Search portals relevant to this space (from index if available)
                let portal_allows = |p: &PortalComponent| -> bool {
                    if !p.is_open { return false; }
                    if !p.shape.segment_intersects(prev.0, t.position) { return false; }
                    // tag filter: require intersection when allow_mask != 0
                    if !(tags_opt.map_or(p.allow_mask == 0, |tm| (tm.mask & p.allow_mask) != 0)) { return false; }
                    // abilities: require that actor has all required bits if any
                    if p.required_abilities_mask != 0 {
                        let actor_mask = abilities_opt.map(|a| a.mask).unwrap_or(0);
                        if (actor_mask & p.required_abilities_mask) != p.required_abilities_mask { return false; }
                    }
                    true
                };
                if let Some(g) = graph.as_ref() {
                    if let Some(list) = g.portals_from.get(space_ent) {
                        for pe in list {
                            if let Ok((_e, p)) = portals.get(*pe) { if portal_allows(p) { permitted = true; to_add.push(p.to); break; } }
                        }
                    }
                    if !permitted {
                        if let Some(list) = g.portals_to.get(space_ent) {
                            for pe in list {
                                if let Ok((_e, p)) = portals.get(*pe) {
                                    if p.bidirectional && p.to == *space_ent && portal_allows(p) { permitted = true; to_add.push(p.from); break; }
                                }
                            }
                        }
                    }
                } else {
                    for (_e, portal) in portals.iter() {
                        if portal.from == *space_ent && portal_allows(portal) { permitted = true; to_add.push(portal.to); break; }
                        if portal.bidirectional && portal.to == *space_ent && portal_allows(portal) { permitted = true; to_add.push(portal.from); break; }
                    }
                }
            }

            // Additional permissive check: if we moved into any neighbor space connected by an allowed, open portal, permit the exit.
            if !permitted {
                let allow_mask_ok = |p: &PortalComponent| {
                    // tags ok
                    let tags_ok = tags_opt.map_or(p.allow_mask == 0, |tm| (tm.mask & p.allow_mask) != 0);
                    // abilities ok
                    let abil_ok = if p.required_abilities_mask == 0 { true } else {
                        let actor_mask = abilities_opt.map(|a| a.mask).unwrap_or(0);
                        (actor_mask & p.required_abilities_mask) == p.required_abilities_mask
                    };
                    tags_ok && abil_ok
                };
                if let Some(g) = graph.as_ref() {
                    // Consider all reachable destinations by allowed, open portals from this source.
                    if let Some(list) = g.portals_from.get(space_ent) {
                        for pe in list {
                            if let Ok((_e, p)) = portals.get(*pe) {
                                if p.is_open && allow_mask_ok(p) { permitted = true; to_add.push(p.to); }
                            }
                        }
                    }
                    if let Some(list) = g.portals_to.get(space_ent) {
                        for pe in list {
                            if let Ok((_e, p)) = portals.get(*pe) {
                                if p.is_open && p.bidirectional && allow_mask_ok(p) { permitted = true; to_add.push(p.from); }
                            }
                        }
                    }
                } else {
                    for (_e, p) in portals.iter() {
                        if p.is_open && allow_mask_ok(p) {
                            if p.from == *space_ent { permitted = true; to_add.push(p.to); }
                            if p.bidirectional && p.to == *space_ent { permitted = true; to_add.push(p.from); }
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

            if permitted {
                exit_events.send(ExitSpaceEvent { entity: ent, space: *space_ent });
                if let Some((_e, sc)) = space_list.iter().find(|(e, _)| e == space_ent) {
                    if sc.kind == crate::components::SpaceKind::Room {
                        acoustics_events.send(AcousticsEvent::RoomExited { entity: ent, room: *space_ent });
                    }
                }
            } else {
                to_retain.push(*space_ent);
            }
        }
        // Retain membership if exit not permitted (apply after iteration to avoid aliasing)
    for s in to_retain { new_set.insert(s); }
    // Add any destination spaces reached via permitted portal crossing, even if medium gating would have filtered them
    for s in to_add { new_set.insert(s); }

        // Medium change detection (Air<->Water) after finalizing membership
        let prev_medium = if space_list.iter().any(|(e, sc)| current.contains(e) && sc.medium == MediumType::Water) { MediumType::Water } else { MediumType::Air };
        let next_medium = if space_list.iter().any(|(e, sc)| new_set.contains(e) && sc.medium == MediumType::Water) { MediumType::Water } else { MediumType::Air };
        if prev_medium != next_medium {
            acoustics_events.send(AcousticsEvent::MediumChanged { entity: ent, from: prev_medium, to: next_medium });
        }

        // Emit Enter events for newly entered spaces using finalized membership set
        for space_ent in new_set.difference(&current) {
            enter_events.send(EnterSpaceEvent { entity: ent, space: *space_ent });
            if let Some((_e, sc)) = space_list.iter().find(|(e, _)| e == space_ent) {
                if sc.kind == crate::components::SpaceKind::Room {
                    acoustics_events.send(AcousticsEvent::RoomEntered { entity: ent, room: *space_ent, material: None });
                }
            }
        }

        // Write back membership component.
        if let Some(mut inside) = inside_opt {
            inside.spaces = new_set;
        } else {
            // Rely on caller to ensure InsideSpaces exists (Engine::ensure_navigation does this).
        }
    }
}

/// Rebuild SpaceGraph indices each frame (could be optimized to change-detection later)
pub fn space_graph_index_system(
    mut graph: ResMut<SpaceGraph>,
    portals: Query<(Entity, &PortalComponent)>,
) {
    graph.portals_from.clear();
    graph.portals_to.clear();
    for (e, p) in portals.iter() {
        graph.portals_from.entry(p.from).or_default().push(e);
        graph.portals_to.entry(p.to).or_default().push(e);
        if p.bidirectional {
            graph.portals_from.entry(p.to).or_default().push(e);
            graph.portals_to.entry(p.from).or_default().push(e);
        }
    }
}

/// Emit boundary proximity cues based on NavMesh
pub fn navmesh_boundary_cues_system(
    navmesh: Option<Res<nm::NavMesh>>,
    mut events: ResMut<Events<crate::events::BoundaryProximityEvent>>,
    q: Query<(Entity, &TransformComponent, Option<&crate::components::NavmeshGuidance>)>,
) {
    let Some(nav) = navmesh else { return; };
    for (e, t, cfg) in q.iter() {
        let Some(idx) = nav.nearest_poly(nm::vec3_to_p2(t.position)) else { continue; };
        let p2 = nav.clamp_to_poly(idx, nm::vec3_to_p2(t.position));
        let dist = nav.boundary_distance(idx, p2);
        if let Some(g) = cfg { if dist <= g.boundary_warn_distance { events.send(crate::events::BoundaryProximityEvent { entity: e, distance: dist }); } }
    }
}

/// Emit simple wayfinding/turn cues: compare current direction to next waypoint vector.
pub fn navmesh_wayfinding_cues_system(
    navmesh: Option<Res<nm::NavMesh>>,
    mut events: ResMut<Events<crate::events::WayfindingCueEvent>>,
    q: Query<(Entity, &TransformComponent, Option<&crate::components::NavmeshGuidance>, Option<&crate::components::NavigationPath>)>,
) {
    let _ = navmesh; // reserved for more advanced occlusion/path smoothing
    for (e, t, cfg, path) in q.iter() {
        let Some(cfg) = cfg else { continue; };
        let Some(path) = path else { continue; };
        if path.index >= path.waypoints.len() { continue; }
        let target = path.waypoints[path.index];
    let to = target - t.position;
        let fwd = glam::Vec3::Z; // assume +Z as forward without a heading component
        let turn = angle_signed_on_y(fwd, to);
        if turn.abs().to_degrees() >= cfg.turn_cue_angle_deg {
            events.send(crate::events::WayfindingCueEvent { entity: e, target, turn });
        }
    }
}

fn angle_signed_on_y(a: glam::Vec3, b: glam::Vec3) -> f32 {
    let a2 = glam::vec2(a.x, a.z).normalize_or_zero();
    let b2 = glam::vec2(b.x, b.z).normalize_or_zero();
    let dot = a2.dot(b2).clamp(-1.0, 1.0);
    let det = a2.perp_dot(b2);
    det.atan2(dot)
}
