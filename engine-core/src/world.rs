use bevy::prelude::*;
use bevy::asset::AssetServer;
use std::collections::HashMap;

use crate::audio::AudioAssets;
use ron::de::from_str;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize, Debug)]
struct WorldDescriptor {
    name: String,
    portals: Option<Vec<PortalDescriptor>>,
    zones: Option<Vec<ZoneDescriptor>>,
    acoustic_volumes: Option<Vec<AcousticVolumeDescriptor>>,
    ambient_sounds: Option<Vec<AmbientSoundDescriptor>>,
}

#[derive(serde::Deserialize, Debug)]
struct PortalDescriptor {
    destination: [f32; 3],
    // AABB in min/max order
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

#[derive(serde::Deserialize, Debug)]
struct ZoneDescriptor {
    name: String,
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

#[derive(serde::Deserialize, Debug)]
struct AcousticVolumeDescriptor {
    name: String,
    reverb_strength: f32,
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

#[derive(serde::Deserialize, Debug)]
struct AmbientSoundDescriptor {
    sound_id: String,
    volume: f32,
    position: [f32; 3],
    looped: Option<bool>,
}

/// Simple resource representing a loaded world map. For now this is a
/// placeholder that may be extended to hold spatial partitions, navmeshes,
/// and entity templates parsed from a file.
#[derive(Resource, Default)]
pub struct WorldMap {
    pub name: Option<String>,
    // future: zones, portals, navmesh, etc.
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldMap::default());
        app.add_systems(Startup, load_assets_and_world);
    }
}

/// On startup, pre-load a handful of audio assets from the workspace `assets/`
/// directory and populate the `AudioAssets` resource. This centralizes asset
/// loading in the core engine instead of application code.
fn load_assets_and_world(mut audio_assets: ResMut<AudioAssets>, asset_server: Res<AssetServer>, mut commands: Commands, mut world_map: ResMut<WorldMap>) {
    // NOTE: Bevy treats paths as relative to the current working directory
    // and may consider `..` paths unapproved. We load from a known `assets/`
    // path inside the workspace to avoid the UnapprovedPath error.
    let preview = asset_server.load("assets/Preview.ogg");
    audio_assets.sounds.insert("preview".to_string(), preview);
    // Try to load a RON-based world descriptor at `assets/worlds/default.world.ron`.
    let path = Path::new("assets/worlds/default.world.ron");
    if path.exists() {
        if let Ok(text) = fs::read_to_string(path) {
            match from_str::<WorldDescriptor>(&text) {
                Ok(desc) => {
                    info!("loaded world descriptor: {}", desc.name);
                    world_map.name = Some(desc.name.clone());
                    if let Some(portals) = desc.portals {
                        for p in portals.iter() {
                            let aabb = crate::components::Aabb { min: bevy::math::Vec3::from(p.aabb_min), max: bevy::math::Vec3::from(p.aabb_max) };
                            let portal = crate::components::Portal { destination: bevy::math::Vec3::from(p.destination), volume_shape: crate::components::VolumeShape::Aabb(aabb) };
                            commands.spawn((portal, crate::components::HasCollider));
                        }
                    }

                    if let Some(zones) = desc.zones {
                        for z in zones.iter() {
                            let aabb = crate::components::Aabb { min: bevy::math::Vec3::from(z.aabb_min), max: bevy::math::Vec3::from(z.aabb_max) };
                            // Use AcousticVolume for zones as well (stores reverb info in default)
                            let vol = crate::components::AcousticVolume { shape: crate::components::VolumeShape::Aabb(aabb), reverb_strength: 0.5 };
                            commands.spawn((vol, crate::components::WorldParent(z.name.clone())));
                        }
                    }

                    if let Some(ac_vols) = desc.acoustic_volumes {
                        for av in ac_vols.iter() {
                            let aabb = crate::components::Aabb { min: bevy::math::Vec3::from(av.aabb_min), max: bevy::math::Vec3::from(av.aabb_max) };
                            let vol = crate::components::AcousticVolume { shape: crate::components::VolumeShape::Aabb(aabb), reverb_strength: av.reverb_strength };
                            commands.spawn((vol, crate::components::WorldParent(av.name.clone())));
                        }
                    }

                    if let Some(amb) = desc.ambient_sounds {
                        for s in amb.iter() {
                            let pos = bevy::math::Vec3::from(s.position);
                            let emitter = crate::components::SoundEmitter { sound_id: s.sound_id.clone(), volume: s.volume, velocity: bevy::math::Vec3::ZERO };
                            commands.spawn((emitter, crate::components::WorldParent(s.sound_id.clone()), Transform::from_translation(pos), GlobalTransform::default()));
                        }
                    }
                }
                Err(e) => warn!("failed to parse world descriptor: {}", e),
            }
        } else {
            warn!("failed to read world descriptor file");
        }
    } else {
        debug!("no world descriptor found at {:?}", path);
    }
}
