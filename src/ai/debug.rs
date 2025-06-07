use bevy::prelude::*;
use oxidized_navigation::debug_draw::DrawNavMesh;
use oxidized_navigation::debug_draw::OxidizedNavigationDebugDrawPlugin;

pub fn plugin(app: &mut App) {
    app.add_plugins((OxidizedNavigationDebugDrawPlugin,));
    app.add_systems(Update, toggle_nav_mesh_debug_draw);
}

/// System for debugging the OxidizedNavigation plugin
fn toggle_nav_mesh_debug_draw(
    keys: Res<ButtonInput<KeyCode>>,
    mut show_navmesh: ResMut<DrawNavMesh>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        show_navmesh.0 = !show_navmesh.0;
        info!("show navmesh: {:?}", show_navmesh.0);
    }
}
