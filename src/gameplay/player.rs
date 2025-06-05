//! Player-specific behavior.

use crate::gameplay::boomerang::ActiveBoomerangThrowOrigin;
use crate::gameplay::camera::CameraFollowTarget;
use crate::gameplay::input::{PlayerActions, PlayerMoveAction};
use crate::screens::Screen;
use avian3d::prelude::{Collider, LinearVelocity, LockedAxes, RigidBody};
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
        LockedAxes::ROTATION_LOCKED,
        MovementSettings { walk_speed: 400. },
        ActiveBoomerangThrowOrigin,
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
    virtual_time: ResMut<Time<Virtual>>,
    real_time: Res<Time<Real>>,
) {
    let camera_transform = camera_query.into_inner();
    let virtual_time = virtual_time.into_inner();
    let real_time = real_time.into_inner();
    let (mut linear_velocity, settings) = player_query.into_inner();
    let camera_right = camera_transform
        .right()
        .as_vec3()
        .with_y(0.)
        .normalize_or_zero();
    let camera_forward = camera_transform
        .forward()
        .as_vec3()
        .with_y(0.)
        .normalize_or_zero();
    let velocity = camera_right * trigger.value.x + camera_forward * trigger.value.y;

    virtual_time.set_relative_speed(velocity.length());

    let final_velocity = velocity.normalize_or_zero()
        * settings.walk_speed
        * Vec3::new(1., 0., 1.)
        * real_time.delta_secs();
    linear_velocity.0 = final_velocity;
}

fn stop_player_directional_input(
    _trigger: Trigger<Completed<PlayerMoveAction>>,
    player: Single<&mut LinearVelocity, With<Player>>,
    virtual_time: ResMut<Time<Virtual>>,
) {
    let virtual_time = virtual_time.into_inner();

    virtual_time.set_relative_speed(0.05);
    let mut player = player.into_inner();
    player.x = 0.;
    player.y = 0.;
    player.z = 0.;
}
