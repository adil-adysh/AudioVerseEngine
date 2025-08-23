#![cfg(feature = "world-loader")]

use bevy_ecs::prelude::*;
use engine_core::world_loader::load_world_from_json;
use engine_core::components::*;
use engine_core::navmesh;

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
            TransformComponent { position: glam::vec3(1.0, 1.0, 1.0), rotation: glam::Quat::IDENTITY, scale: glam::Vec3::ONE, parent: None },
            TraversalMask { mask: u64::MAX },
            Abilities { mask: 0 },
        ))
        .id();

    let id_map = load_world_from_json(&mut world, json.as_bytes()).expect("parse json world");
    assert!(id_map.contains_key("room") && id_map.contains_key("ocean"));
    assert_eq!(id_map.len(), 2);

    // Check resources were inserted
    assert!(world.contains_resource::<TagRegistry>());
    assert!(world.contains_resource::<AbilityRegistry>());
    assert!(world.contains_resource::<navmesh::NavMesh>());

    // Ensure a portal entity with our settings exists
    let mut found = false;
    for p in world.query::<&PortalComponent>().iter(&world) {
        if p.cost == 2.0 { found = true; break; }
    }
    assert!(found, "expected portal with cost 2.0 created");

    // Grant dive and ensure mask updated (registry should have an entry)
    let dive_mask = {
        let mut abil_reg = world.resource_mut::<AbilityRegistry>();
        abil_reg.mask_for(["dive"]) // IntoIterator is implemented for arrays
    };
    world.entity_mut(player).insert(Abilities { mask: dive_mask });
    let ab = world.get::<Abilities>(player).unwrap();
    assert_eq!(ab.mask, dive_mask);
}
