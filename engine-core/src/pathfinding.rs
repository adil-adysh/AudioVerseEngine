use crate::components::SpaceGraph;
use bevy_ecs::prelude::*;

/// Compute a simple path of space centers from `start` to `goal` using A* over the SpaceGraph.
/// Returns a list of waypoints (world positions) or None if no path.
/// Generic A* that works with provided center and neighbor functions.
/// - `center_of(e)` should return the world-space center for a space entity.
/// - `neighbors_of(e)` should return adjacent space entities reachable from `e`.
pub fn astar_spaces<CF, NF>(
    _graph: &SpaceGraph,
    center_of: CF,
    neighbors_of: NF,
    start: Entity,
    goal: Entity,
) -> Option<Vec<glam::Vec3>>
where
    CF: Fn(Entity) -> glam::Vec3,
    NF: Fn(Entity) -> Vec<Entity>,
{
    use std::collections::{BinaryHeap, HashMap, HashSet};
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct Node {
        f: u32,
        space: Entity,
    }
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.f.cmp(&self.f)
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<Entity, u32> = HashMap::new();
    let mut came_from: HashMap<Entity, Entity> = HashMap::new();
    let mut in_open: HashSet<Entity> = HashSet::new();

    let center = |e: Entity| -> glam::Vec3 { center_of(e) };
    let h = |a: Entity, b: Entity| -> u32 {
        let da = center(a) - center(b);
        da.length() as u32
    };

    g_score.insert(start, 0);
    open.push(Node {
        f: h(start, goal),
        space: start,
    });
    in_open.insert(start);

    while let Some(Node { space, .. }) = open.pop() {
        if space == goal {
            break;
        }
        let g_curr = *g_score.get(&space).unwrap_or(&u32::MAX);
        // neighbors via provided neighbor function
        let mut neighbors: Vec<Entity> = neighbors_of(space);
        neighbors.sort_unstable();
        neighbors.dedup();
        for nb in neighbors {
            let tentative = g_curr.saturating_add(1);
            if tentative < *g_score.get(&nb).unwrap_or(&u32::MAX) {
                came_from.insert(nb, space);
                g_score.insert(nb, tentative);
                let f = tentative.saturating_add(h(nb, goal));
                if in_open.insert(nb) {
                    open.push(Node { f, space: nb });
                }
            }
        }
    }

    if !came_from.contains_key(&goal) && start != goal {
        return None;
    }
    // reconstruct
    let mut order: Vec<Entity> = vec![goal];
    let mut cur = goal;
    while let Some(&prev) = came_from.get(&cur) {
        cur = prev;
        order.push(cur);
        if cur == start {
            break;
        }
    }
    order.reverse();
    // map to centers
    Some(order.into_iter().map(center).collect())
}
