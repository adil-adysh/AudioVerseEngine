use bevy_ecs::prelude::*;
use serde::Deserialize;

use crate::components::*;

#[derive(Debug, Deserialize)]
pub struct WorldMap {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub spaces: Vec<SpaceDef>,
    #[serde(default)]
    pub portals: Vec<PortalDef>,
    #[serde(default)]
    pub navmesh: Option<NavMeshDef>,
}

#[derive(Debug, Deserialize)]
pub struct SpaceDef {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub shape: ShapeDef,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub medium: Option<MediumDef>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShapeDef {
    Aabb { min: [f32; 3], max: [f32; 3] },
    Sphere { center: [f32; 3], radius: f32 },
}

impl From<ShapeDef> for Shape3D {
    fn from(s: ShapeDef) -> Self {
        match s {
            ShapeDef::Aabb { min, max } => Shape3D::Aabb(Aabb {
                min: glam::Vec3::from(min),
                max: glam::Vec3::from(max),
            }),
            ShapeDef::Sphere { center, radius } => Shape3D::Sphere(Sphere {
                center: glam::Vec3::from(center),
                radius,
            }),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MediumDef {
    Air,
    Water,
}

#[derive(Debug, Deserialize)]
pub struct PortalDef {
    pub id: String,
    pub from: String,
    pub to: String,
    pub shape: ShapeDef,
    #[serde(default)]
    pub bidirectional: bool,
    #[serde(default = "default_open")]
    pub is_open: bool,
    #[serde(default)]
    pub allow_tags: Vec<String>,
    #[serde(default)]
    pub required_abilities: Vec<String>,
    #[serde(default)]
    pub cost: Option<f32>,
}

fn default_open() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct NavMeshDef {
    #[serde(default)]
    pub polys: Vec<[f32; 4]>,
}

/// Load a JSON world map into the engine world. Returns a map id->Entity for created spaces.
pub fn load_world_from_json(
    world: &mut World,
    json_bytes: &[u8],
) -> anyhow::Result<std::collections::HashMap<String, Entity>> {
    let map: WorldMap = serde_json::from_slice(json_bytes)?;

    // Seed registries
    if !world.contains_resource::<TagRegistry>() {
        world.insert_resource(TagRegistry::default());
    }
    {
        let mut tag_reg = world.resource_mut::<TagRegistry>();
        for t in &map.tags {
            let _ = tag_reg.bit_for(t);
        }
    }
    if !world.contains_resource::<AbilityRegistry>() {
        world.insert_resource(AbilityRegistry::default());
    }
    {
        let mut abil_reg = world.resource_mut::<AbilityRegistry>();
        for a in &map.abilities {
            let _ = abil_reg.bit_for(a);
        }
    }

    // Create spaces first
    let mut id_to_entity: std::collections::HashMap<String, Entity> =
        std::collections::HashMap::new();
    for s in &map.spaces {
        let shape: Shape3D = s.shape.clone().into();
        let tags_mask = {
            let mut tag_reg = world.resource_mut::<TagRegistry>();
            tag_reg.mask_for(s.tags.iter().map(|x| x.as_str()))
        };
        let medium = match s.medium.unwrap_or(MediumDef::Air) {
            MediumDef::Air => MediumType::Air,
            MediumDef::Water => MediumType::Water,
        };
        let e = world
            .spawn(SpaceComponent {
                kind: SpaceKind::Zone,
                name: s.name.clone().unwrap_or(s.id.clone()),
                bounds: shape,
                has_ceiling: true,
                medium,
            })
            .insert(SpaceTags { mask: tags_mask })
            .id();
        id_to_entity.insert(s.id.clone(), e);
    }

    // Create portals
    for p in &map.portals {
        let from = *id_to_entity
            .get(&p.from)
            .ok_or_else(|| anyhow::anyhow!("unknown portal.from {}", p.from))?;
        let to = *id_to_entity
            .get(&p.to)
            .ok_or_else(|| anyhow::anyhow!("unknown portal.to {}", p.to))?;
        let shape: Shape3D = p.shape.clone().into();
        let allow_mask = {
            let mut tag_reg = world.resource_mut::<TagRegistry>();
            tag_reg.mask_for(p.allow_tags.iter().map(|x| x.as_str()))
        };
        let req_abilities = {
            let mut abil_reg = world.resource_mut::<AbilityRegistry>();
            abil_reg.mask_for(p.required_abilities.iter().map(|x| x.as_str()))
        };
        world.spawn(PortalComponent {
            from,
            to,
            shape,
            bidirectional: p.bidirectional,
            is_open: p.is_open,
            allow_mask,
            required_abilities_mask: req_abilities,
            cost: p.cost.unwrap_or(1.0),
        });
    }

    // NavMesh
    let _ = map.navmesh; // No-op: navmesh managed by downstream Bevy plugins now.

    Ok(id_to_entity)
}
