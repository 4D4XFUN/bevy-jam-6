use crate::demo::boomerang::BoomerangHittable;
use crate::demo::enemy::Enemy;
use crate::demo::player::Player;
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{Collider, CollisionLayers, RigidBody};
use bevy::prelude::*;
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;
use rand::{Rng, thread_rng};

pub fn plugin(app: &mut App) {
    app.init_resource::<EnemySpawningConfig>();
    app.add_observer(create_enemy_spawn_points_around_player_on_spawn)
        .add_observer(spawn_enemies_on_enemy_spawn_points);
}

pub fn create_enemy_spawn_points_around_player_on_spawn(
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

    let positions = generate_enemy_spawn_positions(&origin.translation.xy(), &config);

    for p in positions {
        let translation = Vec3::new(p.x, 1.0, p.y); // i think this is right? z is "forward" on our 2d plane in bevy 3d terms, y is skyward
        commands.spawn((EnemySpawnPoint, Transform::from_translation(translation)));
    }

    Ok(())
}

pub fn spawn_enemies_on_enemy_spawn_points(
    trigger: Trigger<OnAdd, EnemySpawnPoint>,
    spawn_points: Query<&Transform, With<EnemySpawnPoint>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let position = spawn_points.get(trigger.target())?;

    commands.spawn((
        Enemy,
        Name::new("Enemy"),
        *position,
        Mesh3d(meshes.add(Capsule3d::default())),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 32, 32))),
        StateScoped(Screen::Gameplay),
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.)),
        BoomerangHittable,
        Collider::capsule(0.5, 1.),
        CollisionLayers::new(GameLayer::Enemy, GameLayer::ALL),
        RigidBody::Dynamic,
    ));

    Ok(())
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

#[derive(Component)]
pub struct EnemySpawnPoint;

pub fn generate_enemy_spawn_positions(origin: &Vec2, config: &EnemySpawningConfig) -> Vec<Vec2> {
    // randomly generate x,y coordinates within the allowed radius and at some random angle from the point of origin

    let n = config.num_enemies;
    let mut rng = thread_rng();

    let mut result = vec![];

    for i in 0..n {
        // Generate random angle (0 to 2π)
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);

        // Generate random radius within the ring
        // Use sqrt for uniform distribution in the annular area
        let min_r_squared = config.min_radius * config.min_radius;
        let max_r_squared = config.max_radius * config.max_radius;
        let radius_squared = rng.gen_range(min_r_squared..max_r_squared);
        let radius = radius_squared.sqrt();

        // Convert polar coordinates to cartesian
        let x = origin.x + (radius * angle.cos()) as f32;
        let y = origin.y + (radius * angle.sin()) as f32;

        result.push(Vec2::new(x, y));
    }

    result
}
