//! Spawn the main level.

use crate::audio::music;
use crate::physics_layers::GameLayer;
use crate::{asset_tracking::LoadResource, screens::Screen};
use avian3d::prelude::CollisionLayers;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<LevelAssets>();
    app.load_resource::<LevelAssets>();
}

/// Todo: maybe add a pub enum LevelSelection
/// and change levels vec into a hashmap of levelSelection and Scene Handles ?
#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency]
    music: Handle<AudioSource>,
    #[dependency]
    pub levels: Vec<Handle<Scene>>,
    pub current_level: usize,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        // add new levels here
        let levels =
            vec![
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Level1.glb")),
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Level2.glb")),
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Level3.glb")),
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Level4.glb")),
            ];
        Self {
            music: asset_server.load("audio/music/BoomerangTheme.ogg"),
            levels,
            current_level: 0,
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(mut commands: Commands, level_assets: ResMut<LevelAssets>) {
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        StateScoped(Screen::Gameplay),
        children![
            (
                Name::new("Gameplay Music"),
                music(level_assets.music.clone()),
            ),
            (
                Name::new("Environment"),
                SceneRoot(level_assets.levels[level_assets.current_level].clone(),),
                CollisionLayers::new(
                    GameLayer::Terrain,
                    [
                        GameLayer::Terrain,
                        GameLayer::Player,
                        GameLayer::Default,
                        GameLayer::Bullet,
                        GameLayer::Enemy,
                        GameLayer::DeadEnemy
                    ]
                ),
            ),
        ],
    ));
}
