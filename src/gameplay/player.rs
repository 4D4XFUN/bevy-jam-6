//! Player-specific behavior.

use crate::gameplay::Gameplay;
use crate::gameplay::ammo::HasLimitedAmmo;
use crate::gameplay::boomerang::CurrentBoomerangThrowOrigin;
use crate::gameplay::camera::CameraFollowTarget;
use crate::gameplay::health_and_damage::{DeathEvent, Health};
use crate::gameplay::input::{PlayerActions, PlayerMoveAction};
use crate::gameplay::score::ScoreEvent;
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{
    AngularVelocity, CoefficientCombine, Collider, CollisionLayers, Friction, LinearVelocity,
    LockedAxes, RigidBody,
};
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
    app.add_systems(OnExit(Gameplay::Normal), teardown);
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
    commands
        .spawn((
            Name::new("Player"),
            Player,
            Transform::from_translation(spawn_point.translation + Vec3::Y),
            Visibility::Inherited,
            Mesh3d(meshes.add(Capsule3d::default())),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 124, 0))),
            StateScoped(Screen::Gameplay),
            MovementSettings::default(),
            CurrentBoomerangThrowOrigin,
            CameraFollowTarget,
        ))
        .insert((
            // add all physics stuff
            Collider::capsule(0.5, 1.),
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED.lock_translation_y(),
            CollisionLayers::new(
                GameLayer::Player,
                [
                    GameLayer::Enemy,
                    GameLayer::Bullet,
                    GameLayer::Terrain,
                    GameLayer::Default,
                ],
            ),
            // We remove friction because we set the velocity each frame anyway
            // also solves problem with weird wall slides
            Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        ))
        .insert((Health::default(), HasLimitedAmmo(1)))
        .observe(on_player_death);
}

fn teardown(player: Single<Entity, With<Player>>, mut commands: Commands) {
    commands
        .entity(player.into_inner())
        .remove::<Actions<PlayerActions>>()
        .remove::<LinearVelocity>()
        .remove::<AngularVelocity>();
}

fn on_player_death(_trigger: Trigger<DeathEvent>, mut commands: Commands) {
    commands.trigger(ScoreEvent::PlayerDeath);
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MovementSettings {
    pub walk_speed: f32,
}
impl Default for MovementSettings {
    fn default() -> Self {
        Self { walk_speed: 6. }
    }
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
) {
    // Rotate input to be on the ground and aligned with camera
    let camera_rotation = camera_query.into_inner().rotation;
    let input_mapped_to_3d = Vec3::new(trigger.value.x, 0.0, -trigger.value.y);
    let velocity = (camera_rotation * input_mapped_to_3d)
        .with_y(0.)
        .normalize_or_zero();

    let (mut linear_velocity, settings) = player_query.into_inner();
    let final_velocity = velocity * settings.walk_speed;
    linear_velocity.0 = final_velocity;
}

fn stop_player_directional_input(
    _trigger: Trigger<Completed<PlayerMoveAction>>,
    player: Single<&mut LinearVelocity, With<Player>>,
) {
    let mut player = player.into_inner();
    player.x = 0.;
    player.y = 0.;
    player.z = 0.;
}
