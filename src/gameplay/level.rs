//! Spawn the main level.

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
    levels: Vec<Handle<Scene>>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        // add new levels here
        let levels =
            vec![asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Environment.gltf"))];
        Self {
            music: asset_server.load("audio/music/Fluffing A Duck.ogg"),
            levels,
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(mut commands: Commands, level_assets: Res<LevelAssets>) {
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        StateScoped(Screen::Gameplay),
        children![
            (
                Name::new("Gameplay Music"),
                // music(_level_assets.music.clone()), // TODO: uncomment to add music back in
            ),
            (
                Name::new("Environment"),
                SceneRoot(level_assets.levels[0].clone(),),
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
