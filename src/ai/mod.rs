mod debug;
pub mod enemy_ai;
pub mod pathfinding_service;

use bevy::prelude::*;
use oxidized_navigation::{
    NavMeshSettings, OxidizedNavigationPlugin, colliders::avian::AvianCollider,
};

pub fn plugin(app: &mut App) {
    // plugins
    app.add_plugins((
        // navmesh_position::plugin,
        pathfinding_service::plugin,
        enemy_ai::plugin,
        debug::plugin,
        OxidizedNavigationPlugin::<AvianCollider>::new(NavMeshSettings::from_agent_and_bounds(
            1.1, 1.9, 1000.0, -1.0,
        )),
    ));
}
