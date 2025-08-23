#![cfg(feature = "world-loader")]

use bevy_ecs::prelude::*;
use engine_core::world_loader::load_world_from_json;
use engine_core::components::*;

#[test]
fn load_world_minimal_and_validate() {
    let json = r#"{
        "version": 1,
        "tags": ["player", "npc"],
        "abilities": ["climb", "dive"],
        "spaces": [
            {"id":"room", "shape": {"type":"aabb", "min":[0,0,0], "max":[10,3,10]}, "tags":["indoor"], "medium":"air"},
            {"id":"ocean", "shape": {"type":"aabb", "min":[20,0,0], "max":[40,5,10]}, "tags":["water"], "medium":"water"}
        ],
        "portals": [
            {"id":"door", "from":"room", "to":"ocean", "shape": {"type":"aabb", "min":[9,0,4], "max":[11,2,6]}, "bidirectional": true, "is_open": true, "allow_tags": ["player"], "required_abilities": ["dive"], "cost": 2.0}
        ],
        "navmesh": {"polys": [[0,0,10,10]]}
    }"#;

    let mut world = World::new();
    // Seed a player entity with components that might be used elsewhere
    let player = world
        .spawn((
            TransformComponent::from_position(glam::Vec3::new(1.0, 1.0, 1.0)),
            TraversalMask { mask: TraversalMask::MASK_ALL },
            Abilities { mask: 0 },
        ))
        .id();

    let id_map = load_world_from_json(&mut world, json.as_bytes()).expect("parse json world");
    assert!(id_map.contains_key("room") && id_map.contains_key("ocean"));
    assert_eq!(id_map.len(), 2);

    // Check resources were inserted
    assert!(world.contains_resource::<TagRegistry>());
    assert!(world.contains_resource::<AbilityRegistry>());
    assert!(world.contains_resource::<crate::navmesh::NavMesh>());

    // Ensure a portal entity with our settings exists
    let mut found = false;
    for p in world.query::<&PortalComponent>().iter(&world) {
        if p.cost == 2.0 { found = true; break; }
    }
    assert!(found, "expected portal with cost 2.0 created");

    // Grant dive and ensure mask updated (registry should have an entry)
    let abil_reg = world.resource::<AbilityRegistry>();
    let dive_mask = abil_reg.mask_for(["dive"].into_iter());
    world.entity_mut(player).insert(Abilities { mask: dive_mask });
    let ab = world.get::<Abilities>(player).unwrap();
    assert_eq!(ab.mask, dive_mask);
}
