use bevy::prelude::*;
use bevy::asset::AssetServer;
use std::collections::HashMap;

use crate::audio::AudioAssets;

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
fn load_assets_and_world(mut audio_assets: ResMut<AudioAssets>, asset_server: Res<AssetServer>) {
    // NOTE: Bevy treats paths as relative to the current working directory
    // and may consider `..` paths unapproved. We load from a known `assets/`
    // path inside the workspace to avoid the UnapprovedPath error.
    let preview = asset_server.load("assets/Preview.ogg");
    audio_assets.sounds.insert("preview".to_string(), preview);

    // Future: try to load `assets/world.json` and parse into WorldMap.
}
