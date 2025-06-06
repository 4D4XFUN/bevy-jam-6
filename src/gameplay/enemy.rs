use crate::asset_tracking::LoadResource;
use crate::gameplay::boomerang::{BOOMERANG_FLYING_HEIGHT, WeaponTarget};
use crate::gameplay::health_and_damage::{CanDamage, DeathEvent};
use crate::gameplay::player::Player;
use crate::gameplay::time_dilation::{DilatedTime, RotationDilated, VelocityDilated};
use crate::gameplay::{boomerang::BoomerangHittable, health_and_damage::Health};
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{
    AngularDamping, Collider, CollisionEventsEnabled, CollisionLayers, Friction, LinearDamping,
    LinearVelocity, Restitution, RigidBody, SpatialQuery, SpatialQueryFilter,
};
use bevy::color;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use rand::{Rng, thread_rng};

pub fn plugin(app: &mut App) {
    app.register_type::<EnemySpawnPoint>();
    app.init_resource::<EnemySpawningConfig>();
    app.load_resource::<PistoleroAssets>();
    app.add_observer(create_enemy_spawn_points_around_player_on_spawn)
        .add_observer(spawn_enemies_on_enemy_spawn_points);
    app.init_gizmo_group::<EnemyAimGizmo>();
    app.add_systems(Update, update_aim_preview_position);
    app.add_systems(Update, attack_target_after_delay);
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct CanUseRangedAttack {
    entity: Entity,
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

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct EnemySpawnPoint;

fn spawn_enemies_on_enemy_spawn_points(
    trigger: Trigger<OnAdd, EnemySpawnPoint>,
    spawn_points: Query<&Transform, With<EnemySpawnPoint>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<Entity, With<Player>>,
) -> Result {
    let position = spawn_points.get(trigger.target())?;
    let player = player_query.single()?;

    let entity = commands
        .spawn((
            Enemy,
            Name::new("Ranged Enemy"),
            *position,
            Mesh3d(meshes.add(Capsule3d::default())),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 32, 32))),
            StateScoped(Screen::Gameplay),
            BoomerangHittable,
            Collider::capsule(0.5, 1.),
            CollisionLayers::new(GameLayer::Enemy, GameLayer::ALL),
            RigidBody::Dynamic,
            Health(1),
        ))
        .observe(on_death)
        .id();
    commands.entity(entity).insert(CanUseRangedAttack {
        entity: player,
        damage: 1,
        max_range: 17.,
        min_range: 2.,
        speed: 20.,
    });
    commands.entity(entity).insert(CanDelayBetweenAttacks {
        timer: Timer::from_seconds(3., TimerMode::Repeating),
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
                gizmos.line(
                    origin_transform.translation.with_y(BOOMERANG_FLYING_HEIGHT),
                    target_location.with_y(BOOMERANG_FLYING_HEIGHT),
                    color::palettes::css::RED,
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
    time: Res<DilatedTime>,
    player_query: Single<&Transform, With<Player>>,
    pistolero_assets: Res<PistoleroAssets>,
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
                Transform::from_translation(origin_transform.translation),
                Bullet,
                SceneRoot(pistolero_assets.bullet.clone()),
                Collider::sphere(0.2),
                CollisionLayers::new(GameLayer::Bullet, [GameLayer::Player, GameLayer::Terrain]),
                RigidBody::Kinematic,
                VelocityDilated(bullet_velocity * ranged_attack.speed),
                RotationDilated(0.),
                CanDamage(1),
                CollisionEventsEnabled,
            ));
            let pitch = rand.r#gen::<f32>() * 0.4;
            commands.spawn((
                AudioPlayer::new(pistolero_assets.gunshot.clone()),
                PlaybackSettings::DESPAWN.with_speed(0.8 + pitch),
            ));
            commands.spawn((
                Name::new("ShellCasing"),
                Transform::from_translation(origin_transform.translation),
                SceneRoot(pistolero_assets.shell.clone()),
                Collider::cylinder(0.05, 0.2),
                RigidBody::Dynamic,
                LinearVelocity(-bullet_velocity * 3.),
                Friction::default(),
                Restitution::default(),
                LinearDamping(0.5),
                AngularDamping(0.5),
            ));
        }
    }
}

fn on_death(_trigger: Trigger<DeathEvent>) {
    info!("ouch! but maybe it'd hurt more if I'd actually die");
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
            num_enemies: 10,
            min_radius: 5.,
            max_radius: 30.,
        }
    }
}

fn create_enemy_spawn_points_around_player_on_spawn(
    trigger: Trigger<OnAdd, Player>,
    query: Query<&Transform, With<Player>>,
    config: Res<EnemySpawningConfig>,
    mut commands: Commands,
) -> Result {
    let origin = query.get(trigger.target())?;
    info!(
        "(dev mode) creating enemy spawners around player at {:?}",
        origin
    );

    // GENERATE ENEMY SPAWN POSITIONS
    let n = config.num_enemies;
    let mut rng = thread_rng();

    let mut positions = vec![];

    for _ in 0..n {
        // Generate random angle (0 to 2π)
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);

        // Generate random radius within the ring
        // Use sqrt for uniform distribution in the annular area
        let min_r_squared = config.min_radius * config.min_radius;
        let max_r_squared = config.max_radius * config.max_radius;
        let radius_squared = rng.gen_range(min_r_squared..max_r_squared);
        let radius = radius_squared.sqrt();

        // Convert polar coordinates to cartesian
        let x = origin.translation.x + (radius * angle.cos()) as f32;
        let y = origin.translation.y + (radius * angle.sin()) as f32;

        positions.push(Vec2::new(x, y));
    }

    for p in positions {
        let translation = Vec3::new(p.x, 1.0, p.y); // i think this is right? z is "forward" on our 2d plane in bevy 3d terms, y is skyward
        commands.spawn((EnemySpawnPoint, Transform::from_translation(translation)));
    }

    Ok(())
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct PistoleroAssets {
    gunshot: Handle<AudioSource>,
    bullet: Handle<Scene>,
    shell: Handle<Scene>,
}

impl FromWorld for PistoleroAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        PistoleroAssets {
            gunshot: asset_server.load("audio/sound_effects/213925__diboz__pistol_riccochet.ogg"),
            bullet: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/bullet.glb")),
            shell: asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/bullet_casing.glb")),
        }
    }
}
