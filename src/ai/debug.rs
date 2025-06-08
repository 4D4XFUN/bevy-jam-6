use crate::ai::enemy_ai::AiMovementState;
use bevy::color::palettes;
use bevy::prelude::*;
use oxidized_navigation::debug_draw::DrawNavMesh;
use oxidized_navigation::debug_draw::OxidizedNavigationDebugDrawPlugin;

pub fn plugin(app: &mut App) {
    app.add_plugins((OxidizedNavigationDebugDrawPlugin,));
    app.add_systems(Update, (toggle_nav_mesh_debug_draw, show_enemy_paths));
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

fn show_enemy_paths(
    query: Query<&AiMovementState>,
    mut gizmos: Gizmos,
    draw_nav_mesh: Res<DrawNavMesh>,
) {
    if !draw_nav_mesh.0 {
        return;
    }

    // debug visualization of enemy paths
    for ai in query.iter() {
        if let AiMovementState::Moving { path, index } = ai {
            gizmos.linestrip(
                path.clone().iter().map(|v| v.with_y(0.2)),
                palettes::css::BLUE,
            );
        }
    }
}
