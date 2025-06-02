//! Spawn the main level.

use bevy::prelude::*;

use crate::{asset_tracking::LoadResource, screens::Screen};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<LevelAssets>();
    app.load_resource::<LevelAssets>();
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency]
    music: Handle<AudioSource>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/Fluffing A Duck.ogg"),
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(
    mut commands: Commands,
    // level_assets: Res<LevelAssets>, // TODO: uncomment to add music back in
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        StateScoped(Screen::Gameplay),
        children![
            (
                Name::new("Gameplay Music"),
                // music(level_assets.music.clone()), // TODO: uncomment to add music back in
            ),
            (
                Name::new("Environment"),
                SceneRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Environment.gltf")),
                ),
            ),
        ],
    ));
}