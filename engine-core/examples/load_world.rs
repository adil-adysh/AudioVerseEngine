// Feature-gated example: run with
//   cargo run -p engine-core --example load_world --features world-loader

#[cfg(feature = "world-loader")]
fn main() {
    use engine_core::engine::Engine;
    use engine_core::world_loader::load_world_from_json;

    let mut engine = Engine::new();
    engine.bootstrap();

    // Minimal JSON world with tags/abilities/spaces/portals
    let json = r#"{
      "version": 1,
      "tags": ["player"],
      "abilities": ["climb"],
      "spaces": [
        {"id": "room_a", "name": "Room A", "shape": {"type": "aabb", "min": [0,0,0], "max": [5,5,5]}, "tags": ["indoors"]},
        {"id": "room_b", "name": "Room B", "shape": {"type": "aabb", "min": [6,0,0], "max": [11,5,5]}, "tags": ["indoors"]}
      ],
      "portals": [
        {"id": "p1", "from": "room_a", "to": "room_b", "shape": {"type": "aabb", "min": [5,0,2], "max": [6,3,3]}, "allow_tags": ["player"], "required_abilities": ["climb"], "cost": 1.0}
      ],
      "navmesh": {"polys": [[0.0,0.0,12.0,6.0]]}
    }"#;

    load_world_from_json(engine.world_mut(), json.as_bytes()).expect("load world");

    println!("Loaded world; registries and navmesh present");
}

#[cfg(not(feature = "world-loader"))]
fn main() {
    eprintln!("Enable feature 'world-loader' to run this example");
}
