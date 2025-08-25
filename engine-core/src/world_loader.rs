// world_loading.rs
// Note: This file is feature-gated: it only compiles when the `world-loader`
// feature is enabled on the `engine-core` crate. This keeps the default build
// lightweight.
#![cfg(feature = "world-loader")]

use std::collections::HashMap;
use bevy_math::Vec3;
use bevy_ecs::prelude::{Commands, Entity, ResMut, Resource};
use serde::Deserialize;
use bevy_transform::components::{Transform, GlobalTransform};
use bevy_rapier3d::prelude::Collider;
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

// Helper: small serde-friendly Vec3 representation (array) to avoid
// requiring serde support on Bevy's Vec3.
#[derive(Debug, Deserialize, Clone)]
pub struct Vec3Def(pub [f32; 3]);
impl Vec3Def {
  fn into_vec3(self) -> Vec3 {
    Vec3::new(self.0[0], self.0[1], self.0[2])
  }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VolumeShapeDef {
  Aabb { min: [f32; 3], max: [f32; 3] },
  Sphere { center: [f32; 3], radius: f32 },
}

impl VolumeShapeDef {
  fn into_volume_shape(self) -> VolumeShape {
    match self {
      VolumeShapeDef::Aabb { min, max } => VolumeShape::Aabb(Aabb { min: Vec3::new(min[0], min[1], min[2]), max: Vec3::new(max[0], max[1], max[2]) }),
      VolumeShapeDef::Sphere { center, radius } => VolumeShape::Sphere(Sphere { center: Vec3::new(center[0], center[1], center[2]), radius }),
    }
  }
}

// Deserializable component payloads (small, serde-friendly) which are
// converted to the real engine components at spawn time.
#[derive(Debug, Deserialize)]
pub struct SoundEmitterDef {
  pub sound_id: String,
  #[serde(default = "default_one")]
  pub volume: f32,
  #[serde(default)]
  pub velocity: Option<[f32; 3]>,
}

fn default_one() -> f32 { 1.0 }

#[derive(Debug, Deserialize)]
pub struct SoundMaterialDef {
  #[serde(default = "default_zero")]
  pub occlusion_strength: f32,
  #[serde(default = "default_zero")]
  pub reflectivity: f32,
}

fn default_zero() -> f32 { 0.0 }

#[derive(Debug, Deserialize)]
pub struct AcousticVolumeDef {
  pub shape: VolumeShapeDef,
  #[serde(default = "default_zero")]
  pub reverb_strength: f32,
}

#[derive(Debug, Deserialize)]
pub struct MediumVolumeDef {
  pub shape: VolumeShapeDef,
  #[serde(default = "default_medium")]
  pub medium_type: String,
}

fn default_medium() -> String { "air".to_string() }

#[derive(Debug, Deserialize)]
pub struct PortalDef {
  pub destination: [f32; 3],
  pub volume_shape: VolumeShapeDef,
}

// An enum that can represent any component that can be loaded from our JSON.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComponentDef {
  // Player and Movement
  Player,
  MovementSpeed(f32),
  MoveDirection([f32; 3]),
  HasCollider,
  // Audio (use helper defs)
  SoundEmitter(SoundEmitterDef),
  SoundMaterial(SoundMaterialDef),
  // Environmental Volumes
  AcousticVolume(AcousticVolumeDef),
  MediumVolume(MediumVolumeDef),
  // Portal System
  Portal(PortalDef),
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
        ComponentDef::MoveDirection(dir) => { entity_commands.insert(MoveDirection(Vec3::new(dir[0], dir[1], dir[2]))); },
        ComponentDef::HasCollider => {
          // Insert a default, conservative collider so entities with the
          // HasCollider marker participate in physics queries. The collider
          // shape here is intentionally conservative and can be overridden
          // by application-specific components or subsequent systems.
          entity_commands.insert(HasCollider);
          entity_commands.insert(Collider::cuboid(0.5, 1.0, 0.5));
        },
        ComponentDef::SoundEmitter(s) => {
          let vel = s.velocity.unwrap_or([0.0, 0.0, 0.0]);
          entity_commands.insert(SoundEmitter { sound_id: s.sound_id, volume: s.volume, velocity: Vec3::new(vel[0], vel[1], vel[2]) });
        },
        ComponentDef::SoundMaterial(s) => {
          entity_commands.insert(SoundMaterial { occlusion_strength: s.occlusion_strength, reflectivity: s.reflectivity });
        },
        ComponentDef::AcousticVolume(s) => {
          entity_commands.insert(AcousticVolume { shape: s.shape.clone().into_volume_shape(), reverb_strength: s.reverb_strength });
        },
        ComponentDef::MediumVolume(s) => {
          let medium = match s.medium_type.as_str() {
            "water" => MediumType::Water,
            _ => MediumType::Air,
          };
          entity_commands.insert(MediumVolume { shape: s.shape.clone().into_volume_shape(), medium_type: medium });
        },
        ComponentDef::Portal(p) => {
          // Insert the Portal component so the portal systems can use the
          // descriptor, and also create a Rapier collider matching the
          // portal's volume shape so physics queries and raycasts can hit it.
          let portal = Portal { destination: Vec3::new(p.destination[0], p.destination[1], p.destination[2]), volume_shape: p.volume_shape.clone().into_volume_shape() };
          // If the portal has an AABB or Sphere volume shape we translate
          // the spawned entity to the shape center and attach a collider
          // with matching half-extents / radius.
          match portal.volume_shape {
            VolumeShape::Aabb(aabb) => {
              let center = (aabb.min + aabb.max) * 0.5;
              let half = (aabb.max - aabb.min) * 0.5;
              entity_commands.insert(portal.clone());
              entity_commands.insert(Transform::from_translation(center));
              entity_commands.insert(Collider::cuboid(half.x.abs(), half.y.abs(), half.z.abs()));
            }
            VolumeShape::Sphere(sphere) => {
              entity_commands.insert(portal.clone());
              entity_commands.insert(Transform::from_translation(sphere.center));
              entity_commands.insert(Collider::ball(sphere.radius.abs()));
            }
          }
        },
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

