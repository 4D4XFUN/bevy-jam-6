use bevy::prelude::*;
use bevy::prelude::*;
use oxidized_navigation::{
    colliders::avian::AvianCollider, debug_draw::OxidizedNavigationDebugDrawPlugin,
    NavMeshSettings,
    OxidizedNavigationPlugin,
};

pub fn plugin(app: &mut App) {
    app.add_plugins((
        OxidizedNavigationPlugin::<AvianCollider>::new(NavMeshSettings::from_agent_and_bounds(
            0.5, 1.9, 250.0, -1.0,
        )),
        OxidizedNavigationDebugDrawPlugin,
    ));
}
