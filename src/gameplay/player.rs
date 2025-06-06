//! Player-specific behavior.

use crate::gameplay::boomerang::ActiveBoomerangThrowOrigin;
use crate::gameplay::camera::CameraFollowTarget;
use crate::gameplay::input::{PlayerActions, PlayerMoveAction};
use crate::gameplay::time_dilation::DilatedTime;
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{Collider, CollisionLayers, LinearVelocity, LockedAxes, RigidBody};
use bevy::prelude::*;
use bevy_enhanced_input::events::Completed;
use bevy_enhanced_input::prelude::{Actions, Fired};

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PlayerSpawnPoint;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Player>()
        .register_type::<PlayerSpawnPoint>();

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
        Transform::from_translation(spawn_point.translation + Vec3::Y),
        Visibility::Inherited,
        Mesh3d(meshes.add(Capsule3d::default())),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 124, 0))),
        Collider::capsule(0.5, 1.),
        StateScoped(Screen::Gameplay),
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED.lock_translation_y(),
        MovementSettings { walk_speed: 400. },
        ActiveBoomerangThrowOrigin,
        CollisionLayers::new(
            GameLayer::Player,
            [
                GameLayer::Enemy,
                GameLayer::Bullet,
                GameLayer::Terrain,
                GameLayer::Default,
            ],
        ),
        CameraFollowTarget, // Can't add more components to this tuple, it is at max capacity, we should use the insert component command on the entity
    ));
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MovementSettings {
    walk_speed: f32,
}

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
    trigger: Trigger<Fired<PlayerMoveAction>>,
    player_query: Single<
        (&mut LinearVelocity, &MovementSettings),
        (With<Player>, Without<Camera3d>),
    >,
    camera_query: Single<&Transform, With<Camera3d>>,
    time: ResMut<DilatedTime>,
) {
    // Rotate input to be on the ground and aligned with camera
    let camera_rotation = camera_query.into_inner().rotation;
    let input_mapped_to_3d = Vec3::new(trigger.value.x, 0.0, -trigger.value.y);
    let velocity = (camera_rotation * input_mapped_to_3d)
        .with_y(0.)
        .normalize_or_zero();

    let (mut linear_velocity, settings) = player_query.into_inner();
    let final_velocity = velocity * settings.walk_speed * time.delta.as_secs_f32();
    linear_velocity.0 = final_velocity;
}

fn stop_player_directional_input(
    _trigger: Trigger<Completed<PlayerMoveAction>>,
    player: Single<&mut LinearVelocity, With<Player>>,
    time: ResMut<DilatedTime>,
) {
    let mut player = player.into_inner();
    player.x = 0.;
    player.y = 0.;
    player.z = 0.;
}
