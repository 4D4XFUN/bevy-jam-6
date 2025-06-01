//! Player-specific behavior.

use crate::demo::input::{PlayerActions, PlayerMove};
use crate::screens::Screen;
use crate::{asset_tracking::LoadResource, demo::movement::MovementController};
use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};
use bevy_enhanced_input::events::Completed;
use bevy_enhanced_input::prelude::{Actions, Fired};

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PlayerSpawnPoint;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Player>()
        .register_type::<PlayerSpawnPoint>();

    app.register_type::<PlayerAssets>();
    app.load_resource::<PlayerAssets>();
    app.add_observer(spawn_player_to_point);
    // we attach movement-related observers to the player entity so that they
    // get despawned when the player does. That way, movement happens only while
    // playing, not while e.g. in a menu or splash screen.
    app.add_observer(add_player_movement_on_spawn);
}

fn spawn_player_to_point(
    trigger: Trigger<OnAdd, PlayerSpawnPoint>,
    spawn_points: Query<&Transform, With<PlayerSpawnPoint>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let Ok(spawn_point) = spawn_points.get(trigger.target()) else {
        warn!("No spawn point found!");
        return;
    };
    info!("spawn point at {:?} added", spawn_point);
    commands.spawn((
        Name::new("Player"),
        Player,
        Transform::from_translation(spawn_point.translation),
        children![(
            Mesh3d(meshes.add(Capsule3d::default())),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 124, 0))),
            Transform::from_xyz(0., 1.0, 0.)
        )],
        StateScoped(Screen::Gameplay),
        MovementController { ..default() },
    ));
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
struct Player;

fn add_player_movement_on_spawn(
    trigger: Trigger<OnAdd, Player>,
    query: Query<Entity, With<Player>>,
    mut commands: Commands,
) -> Result {
    let id = query.get(trigger.target())?;
    commands
        .entity(id)
        .insert(Actions::<PlayerActions>::default())
        .observe(record_player_directional_input)
        .observe(stop_player_directional_input);
    Ok(())
}

fn record_player_directional_input(
    trigger: Trigger<Fired<PlayerMove>>,
    mut movement_controller: Query<&mut MovementController>,
) -> Result {
    movement_controller.get_mut(trigger.target())?.intent = trigger.value; // vector is already normalized for us
    Ok(())
}

fn stop_player_directional_input(
    trigger: Trigger<Completed<PlayerMove>>,
    mut movement_controller: Query<&mut MovementController>,
) -> Result {
    movement_controller.get_mut(trigger.target())?.intent = Vec2::ZERO;
    Ok(())
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct PlayerAssets {
    #[dependency]
    ducky: Handle<Image>,
    #[dependency]
    pub steps: Vec<Handle<AudioSource>>,
}

impl FromWorld for PlayerAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            ducky: assets.load_with_settings(
                "images/ducky.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            steps: vec![
                assets.load("audio/sound_effects/step1.ogg"),
                assets.load("audio/sound_effects/step2.ogg"),
                assets.load("audio/sound_effects/step3.ogg"),
                assets.load("audio/sound_effects/step4.ogg"),
            ],
        }
    }
}
