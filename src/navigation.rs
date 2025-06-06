use crate::physics_layers::GameLayer;
use avian3d::prelude::{Collider, CollisionLayers, RigidBody};
use bevy::math::primitives;
use bevy::prelude::*;
use bevy::prelude::*;
use oxidized_navigation::debug_draw::DrawNavMesh;
use oxidized_navigation::{
    NavMeshAffector, NavMeshSettings, OxidizedNavigationPlugin, colliders::avian::AvianCollider,
    debug_draw::OxidizedNavigationDebugDrawPlugin,
};

pub fn plugin(app: &mut App) {
    app.add_plugins((
        OxidizedNavigationDebugDrawPlugin,
        OxidizedNavigationPlugin::<AvianCollider>::new(NavMeshSettings::from_agent_and_bounds(
            0.5, 1.9, 250.0, -1.0,
        )),
    ));

    // app.add_systems(Startup, spawn_test_entities);
    app.add_systems(Update, toggle_nav_mesh_debug_draw);
}

fn toggle_nav_mesh_debug_draw(
    keys: Res<ButtonInput<KeyCode>>,
    mut show_navmesh: ResMut<DrawNavMesh>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        show_navmesh.0 = !show_navmesh.0;
        info!("show navmesh: {:?}", show_navmesh.0);
    }
}

// this is throwaway code to test that all the dependencies are working
fn spawn_test_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(25.0, 25.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, 0.01, 0.0), // raise slightly to prevent flicker
        Collider::cuboid(25.0, 0.1, 25.0),
        NavMeshAffector, // Only entities with a NavMeshAffector component will contribute to the nav-mesh.
        CollisionLayers::new(
            GameLayer::Terrain,
            [
                GameLayer::Player,
                GameLayer::Enemy,
                GameLayer::Bullet,
                GameLayer::Default,
            ],
        ),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(primitives::Cuboid::new(2.5, 2.5, 2.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.1, 0.5))),
        Transform::from_xyz(-5.0, 0.8, -5.0),
        Collider::cuboid(1.25, 1.25, 1.25),
        NavMeshAffector, // Only entities with a NavMeshAffector component will contribute to the nav-mesh.
        RigidBody::Static,
        CollisionLayers::new(
            GameLayer::Terrain,
            [
                GameLayer::Player,
                GameLayer::Enemy,
                GameLayer::Bullet,
                GameLayer::Default,
            ],
        ),
    ));

    // Thin wall
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(primitives::Cuboid::new(0.1, 0.1, 0.1)))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.1, 0.5))),
        Transform::from_xyz(-3.0, 0.8, 5.0).with_scale(Vec3::new(50.0, 15.0, 1.0)),
        Collider::cuboid(0.05, 0.05, 0.05),
        NavMeshAffector, // Only entities with a NavMeshAffector component will contribute to the nav-mesh.
        RigidBody::Static,
        CollisionLayers::new(
            GameLayer::Terrain,
            [
                GameLayer::Player,
                GameLayer::Enemy,
                GameLayer::Bullet,
                GameLayer::Default,
            ],
        ),
    ));
}
