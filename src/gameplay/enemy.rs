use crate::ai::enemy_ai::{AiMovementState, FollowPlayerBehavior};
use crate::asset_tracking::LoadResource;
use crate::audio::TimeDilatedPitch;
use crate::gameplay::Gameplay;
use crate::gameplay::boomerang::{BOOMERANG_FLYING_HEIGHT, WeaponTarget};
use crate::gameplay::health_and_damage::{CanDamage, DeathEvent};
use crate::gameplay::player::Player;
use crate::gameplay::score::ScoreEvent;
use crate::gameplay::{boomerang::BoomerangHittable, health_and_damage::Health};
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{
    AngularDamping, AngularVelocity, Collider, CollisionEventsEnabled, CollisionLayers, Friction,
    LinearDamping, LinearVelocity, LockedAxes, Physics, PhysicsLayer, Restitution, RigidBody,
    SpatialQuery, SpatialQueryFilter,
};
use bevy::color;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use rand::{Rng, thread_rng};

pub fn plugin(app: &mut App) {
    app.register_type::<EnemySpawnPoint>();
    app.init_resource::<EnemySpawningConfig>();
    app.load_resource::<PistoleroAssets>();
    app.add_observer(spawn_enemies_on_enemy_spawn_points);
    app.init_gizmo_group::<EnemyAimGizmo>();
    app.add_systems(
        Update,
        (update_aim_preview_position, attack_target_after_delay).run_if(in_state(Gameplay::Normal)),
    );
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct CanUseRangedAttack {
    damage: usize,
    max_range: f32,
    min_range: f32,
    speed: f32,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
struct CanDelayBetweenAttacks {
    timer: Timer,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct EnemyAimGizmo;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PlayerSeeking;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Enemy;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Bullet;

#[derive(Component, Reflect, Copy, Clone, Debug, PartialEq)]
#[reflect(Default, Component)]
pub struct EnemySpawnPoint {
    detection_range: f32,
    movement_speed: f32,
    attacks_per_second: f32,
}

impl Default for EnemySpawnPoint {
    fn default() -> Self {
        let default_ai = FollowPlayerBehavior::default();
        Self {
            detection_range: default_ai.detection_range,
            movement_speed: default_ai.movement_speed,
            attacks_per_second: 1.0,
        }
    }
}

fn spawn_enemies_on_enemy_spawn_points(
    trigger: Trigger<OnAdd, EnemySpawnPoint>,
    spawn_points: Query<(&Transform, &EnemySpawnPoint)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let (position, spawn_point) = spawn_points.get(trigger.target())?;

    let entity = commands
        .spawn((
            Enemy,
            Name::new("Ranged Enemy"),
            FollowPlayerBehavior {
                detection_range: spawn_point.detection_range,
                movement_speed: spawn_point.movement_speed,
                ..default()
            },
            *position,
            Mesh3d(meshes.add(Capsule3d::default())),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 32, 32))),
            StateScoped(Screen::Gameplay),
            BoomerangHittable,
            Collider::capsule(0.5, 1.),
            CollisionLayers::new(
                GameLayer::Enemy,
                [
                    GameLayer::Player,
                    GameLayer::Boomerang,
                    // GameLayer::Terrain,
                    // GameLayer::Default,
                ],
            ),
            LinearVelocity::ZERO,
            LockedAxes::ROTATION_LOCKED.lock_translation_y(),
            RigidBody::Kinematic,
            Health(1),
        ))
        .observe(on_death)
        .id();
    commands.entity(entity).insert(CanUseRangedAttack {
        damage: 1,
        max_range: 17.,
        min_range: 2.,
        speed: 20.,
    });
    commands.entity(entity).insert(CanDelayBetweenAttacks {
        timer: Timer::from_seconds(1.0 / spawn_point.attacks_per_second, TimerMode::Repeating), // todo revert cooldown when done testing navmesh stuff
    });
    commands.entity(entity).insert(WeaponTarget {
        target_entity: None,
    });

    Ok(())
}

fn update_aim_preview_position(
    mut attacker_query: Query<(Entity, &Transform, &CanUseRangedAttack, &mut WeaponTarget)>,
    player_query: Single<(Entity, &Transform), With<Player>>,
    spatial_query: SpatialQuery,
    mut gizmos: Gizmos<EnemyAimGizmo>,
) {
    let (player_entity, player_transform) = player_query.into_inner();
    let player_translation = player_transform.translation;

    for (origin_entity, origin_transform, can_use_ranged_attack, mut weapon_target) in
        attacker_query.iter_mut()
    {
        let origin = origin_transform.translation.with_y(BOOMERANG_FLYING_HEIGHT);

        let Some(direction) = (player_translation - origin).try_normalize() else {
            return;
        };

        let max_distance = can_use_ranged_attack.max_range;
        let solid = true;
        let filter = SpatialQueryFilter {
            excluded_entities: EntityHashSet::from([origin_entity]),
            ..Default::default()
        };
        if let Some(first_hit) = spatial_query.cast_ray(
            origin,
            Dir3::new_unchecked(direction),
            max_distance,
            solid,
            &filter,
        ) {
            if first_hit.entity == player_entity {
                let target_location = origin + direction * first_hit.distance;

                let aiming_line_length = 1.;
                let aim_line_scaled_direction = (target_location - origin_transform.translation)
                    .normalize_or_zero()
                    * aiming_line_length;
                let aim_line_endpoint = origin_transform.translation + aim_line_scaled_direction;

                gizmos.line(
                    origin_transform.translation.with_y(BOOMERANG_FLYING_HEIGHT),
                    aim_line_endpoint,
                    color::palettes::css::RED.with_alpha(0.5),
                );
                weapon_target.target_entity = Some(player_entity);
            } else {
                weapon_target.target_entity = None;
            }
        } else {
            weapon_target.target_entity = None;
        }
    }
}

fn attack_target_after_delay(
    mut commands: Commands,
    mut attacker_query: Query<
        (
            &CanUseRangedAttack,
            &Transform,
            &WeaponTarget,
            &mut CanDelayBetweenAttacks,
        ),
        With<Enemy>,
    >,
    time: Res<Time<Physics>>,
    player_query: Single<&Transform, With<Player>>,
    pistolero_assets: Res<PistoleroAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rand = thread_rng();
    let player_transform = player_query.into_inner();
    for (ranged_attack, origin_transform, attacker_target, mut can_delay) in
        attacker_query.iter_mut()
    {
        can_delay.timer.tick(time.delta());
        if can_delay.timer.just_finished() && attacker_target.target_entity.is_some() {
            let bullet_velocity =
                (player_transform.translation - origin_transform.translation).normalize_or_zero();

            commands.spawn((
                Name::new("Bullet"),
                Transform::from_translation(origin_transform.translation)
                    .with_scale(Vec3::new(2., 2., 2.)),
                Bullet,
                SceneRoot(pistolero_assets.bullet.clone()),
                MeshMaterial3d(materials.add(Color::srgb_u8(50, 0, 0))),
                Collider::sphere(0.1),
                CollisionLayers::new(GameLayer::Bullet, [GameLayer::Player, GameLayer::Terrain]),
                RigidBody::Kinematic,
                LinearVelocity(bullet_velocity * ranged_attack.speed),
                CanDamage(1),
                CollisionEventsEnabled,
                StateScoped(Screen::Gameplay),
            ));
            let pitch = rand.r#gen::<f32>() * 0.4;
            commands.spawn((
                Name::from("Gunshot SFX"),
                AudioPlayer::new(pistolero_assets.gunshot.clone()),
                PlaybackSettings::DESPAWN,
                TimeDilatedPitch(0.8 + pitch),
            ));
            commands.spawn((
                Name::new("ShellCasing"),
                Transform::from_translation(origin_transform.translation),
                SceneRoot(pistolero_assets.shell.clone()),
                Collider::cylinder(0.05, 0.2),
                CollisionLayers::new(GameLayer::DeadEnemy, GameLayer::Terrain),
                RigidBody::Dynamic,
                LinearVelocity(-bullet_velocity * 3.),
                Friction::default(),
                Restitution::default(),
                LinearDamping(0.5),
                AngularDamping(0.5),
                StateScoped(Screen::Gameplay),
            ));
        }
    }
}

fn on_death(
    trigger: Trigger<DeathEvent>,
    pistolero_assets: Res<PistoleroAssets>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .entity(trigger.target())
        .remove::<CanUseRangedAttack>()
        .remove::<FollowPlayerBehavior>()
        .remove::<AiMovementState>()
        .remove::<LockedAxes>()
        .insert(RigidBody::Dynamic)
        .insert(MeshMaterial3d(materials.add(Color::srgb_u8(240, 200, 200))))
        .insert(LinearVelocity::from(Vec3::new(3., 3., 3.))) // This is temp, we should move the dead thing in the opposite direction of the hit.
        .insert(AngularVelocity::from(Vec3::new(3., 3., 3.))) // This is temp, we should move the dead thing in the opposite direction of the hit.
        .insert(LinearDamping(0.5))
        .insert(AngularDamping(0.5))
        .insert(CollisionLayers::new(
            GameLayer::DeadEnemy,
            GameLayer::all_bits(),
        ));
    let multiplicator = trigger.event().0 as f32;
    commands.trigger(ScoreEvent::AddScore(100. * multiplicator));
    commands.trigger(ScoreEvent::EnemyDeath);
    let rand = thread_rng().gen_range(0..pistolero_assets.death_screams.len());
    commands.spawn((
        Name::from("DeathScream"),
        AudioPlayer::new(pistolero_assets.death_screams[rand].clone()),
        PlaybackSettings::DESPAWN,
        TimeDilatedPitch(1.0),
    ));
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct EnemySpawningConfig {
    num_enemies: usize,
    min_radius: f64,
    max_radius: f64,
}

impl Default for EnemySpawningConfig {
    fn default() -> Self {
        Self {
            num_enemies: 0,
            min_radius: 5.,
            max_radius: 30.,
        }
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct PistoleroAssets {
    gunshot: Handle<AudioSource>,
    bullet: Handle<Scene>,
    shell: Handle<Scene>,
    death_screams: Vec<Handle<AudioSource>>,
}

impl FromWorld for PistoleroAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let death_scream = vec![
            asset_server.load("audio/sound_effects/Wilhelm-ScreamSFX/Wilhelm 1.ogg"),
            asset_server.load("audio/sound_effects/Wilhelm-ScreamSFX/Wilhelm 2.ogg"),
            asset_server.load("audio/sound_effects/Wilhelm-ScreamSFX/Wilhelm 3.ogg"),
            asset_server.load("audio/sound_effects/Wilhelm-ScreamSFX/Wilhelm 4.ogg"),
            asset_server.load("audio/sound_effects/Wilhelm-ScreamSFX/Wilhelm 5.ogg"),
            asset_server.load("audio/sound_effects/Wilhelm-ScreamSFX/Wilhelm 6.ogg"),
        ];
        PistoleroAssets {
            gunshot: asset_server.load("audio/sound_effects/213925__diboz__pistol_riccochet.ogg"),
            bullet: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/bullet.glb")),
            shell: asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/bullet_casing.glb")),
            death_screams: death_scream,
        }
    }
}
