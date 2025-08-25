// world_loading.rs
// Note: This file would normally be separate. Included here for a complete example.
use std::collections::HashMap;
use bevy_math::Vec3;
use bevy_ecs::prelude::{Commands, Entity, ResMut, Resource};
use serde::Deserialize;
use bevy_transform::components::{Transform, GlobalTransform};
use crate::components::*;

// Data structure for deserializing the entire world map from JSON.
// It uses a flat list of entities with a parent_id to build the hierarchy.
#[derive(Debug, Deserialize)]
pub struct WorldMap {
    pub entities: Vec<EntityDef>,
}

#[derive(Debug, Deserialize)]
pub struct EntityDef {
    pub id: String,
  #[serde(default)]
    pub parent_id: Option<String>,
    pub components: Vec<ComponentDef>,
}

// An enum that can represent any component that can be loaded from our JSON.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComponentDef {
    // Player and Movement
    Player,
    MovementSpeed(f32),
    MoveDirection(Vec3),
    HasCollider,
    // Physics
    // (Velocity and ExternalForce are not here as they are runtime-only)
    // Audio
    SoundEmitter(SoundEmitter),
    SoundMaterial(SoundMaterial),
    // Environmental Volumes
    AcousticVolume(AcousticVolume),
    MediumVolume(MediumVolume),
    // Portal System
    Portal(Portal),
}

// For this example, we'll use a hardcoded JSON string. In a full game,
// this would be loaded from a file asset.
const WORLD_MAP_JSON: &str = r#"
{
  "entities": [
    {
      "id": "player_spawn",
      "components": [
        { "type": "player" },
        { "type": "movement_speed", "0": 5.0 },
        { "type": "has_collider" }
      ]
    },
    {
      "id": "house_1",
      "components": [
        { "type": "acoustic_volume", "shape": { "type": "aabb", "min": [-5, 0, -5], "max": [5, 5, 5] }, "reverb_strength": 0.8 }
      ]
    },
    {
      "id": "living_room",
      "parent_id": "house_1",
      "components": [
        { "type": "medium_volume", "shape": { "type": "aabb", "min": [-4, 0, -4], "max": [4, 4, 4] }, "medium_type": "air" }
      ]
    },
    {
        "id": "window_portal",
        "parent_id": "house_1",
        "components": [
            { "type": "portal", "destination": [20.0, 1.0, 20.0], "volume_shape": { "type": "aabb", "min": [0.0, 1.0, 5.0], "max": [1.0, 2.0, 5.0] } }
        ]
    }
  ]
}
"#;

// A dedicated resource to store the map from string IDs to Bevy Entity IDs.
#[derive(Resource, Debug, Default)]
pub struct EntityIdMap(pub HashMap<String, Entity>);

// The system to load the world from our JSON data.
pub fn load_world_from_json_system(mut commands: Commands, mut id_map: ResMut<EntityIdMap>) {
    println!("Loading world from JSON...");

    let world_map: WorldMap = match serde_json::from_str(WORLD_MAP_JSON) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("Failed to deserialize world map: {}", e);
            return;
        }
    };

  // Pass 1: Spawn all entities and insert components.
  // We store only id and parent_id to apply parenting in the second pass.
  let mut entity_defs_to_parent: Vec<(String, Option<String>)> = Vec::new();
  for entity_def in world_map.entities {
    let mut entity_commands = commands.spawn((Transform::default(), GlobalTransform::default()));
    id_map.0.insert(entity_def.id.clone(), entity_commands.id());

    for component_def in entity_def.components {
      match component_def {
        ComponentDef::Player => { entity_commands.insert(Player); },
        ComponentDef::MovementSpeed(speed) => { entity_commands.insert(MovementSpeed(speed)); },
        ComponentDef::MoveDirection(dir) => { entity_commands.insert(MoveDirection(dir)); },
        ComponentDef::HasCollider => { entity_commands.insert(HasCollider); },
        ComponentDef::SoundEmitter(s) => { entity_commands.insert(s); },
        ComponentDef::SoundMaterial(s) => { entity_commands.insert(s); },
        ComponentDef::AcousticVolume(s) => { entity_commands.insert(s); },
        ComponentDef::MediumVolume(s) => { entity_commands.insert(s); },
        ComponentDef::Portal(p) => { entity_commands.insert(p); },
      }
    }
    entity_defs_to_parent.push((entity_def.id.clone(), entity_def.parent_id.clone()));
  }

    // Pass 2: Re-iterate and apply parenting using the ID map.
  for (id, parent_id) in entity_defs_to_parent {
    if let Some(parent_id) = parent_id {
      if let Some(&parent_bevy_id) = id_map.0.get(&parent_id) {
        if let Ok(mut entity_commands) = commands.get_entity(id_map.0[&id]) {
          // Use the existing set_parent method; it's deprecated but
          // still functional and avoids adding new type imports.
          entity_commands.set_parent(parent_bevy_id);
        } else {
          eprintln!("Parenting failed: Could not find entity with ID '{}'", id);
        }
      } else {
        eprintln!("Parenting failed: Parent with ID '{}' not found for entity '{}'", parent_id, id);
      }
    }
  }
    println!("World loaded successfully!");
}

