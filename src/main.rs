// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

mod asset_tracking;
mod assets;
mod audio;
mod demo;
#[cfg(feature = "dev")]
mod dev_tools;
mod physics_layers;
mod screens;
mod settings;
mod theme;

use avian3d::PhysicsPlugins;
use bevy::window::PresentMode;
use bevy::{asset::AssetMetaCheck, prelude::*};
use bevy_skein::SkeinPlugin;
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Order new `AppSystems` variants by adding them here:
        app.configure_sets(
            Update,
            (
                AppSystems::TickTimers,
                AppSystems::RecordInput,
                AppSystems::Update,
            )
                .chain(),
        );

        // Spawn the main camera.
        // app.add_systems(Startup, spawn_camera);

        // Add Bevy plugins.
        app.add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "A FISTFUL OF BOOMERANGS".to_string(),
                        present_mode: PresentMode::AutoNoVsync,
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
            SkeinPlugin::default(),
            PhysicsPlugins::default(),
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ));

        // Add other plugins.
        app.add_plugins((
            asset_tracking::plugin,
            demo::plugin,
            #[cfg(feature = "dev")]
            dev_tools::plugin,
            screens::plugin,
            theme::plugin,
            settings::plugin,
        ));
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
/// When adding a new variant, make sure to order it in the `configure_sets`
/// call above.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum AppSystems {
    /// Tick timers.
    TickTimers,
    /// Record player input.
    RecordInput,
    /// Do everything else (consider splitting this into further variants).
    Update,
}

// bitflags! { //removed this until we need it for the 3d camera
//     struct RenderLayer: u32 {
//         /// Used implicitly by all entities without a `RenderLayers` component.
//         /// Our world model camera and all objects other than the player are on this layer.
//         /// The light source belongs to both layers.
//         const DEFAULT = 0b00000001;
//         /// Since we use multiple cameras, we need to be explicit about
//         /// which one is allowed to render particles.
//         const PARTICLES = 0b00000010;
//         /// 3D gizmos. These need to be rendered only by a 3D camera, otherwise the UI camera will render them in a buggy way.
//         /// Specifically, the UI camera is a 2D camera, which by default is placed at a far away Z position,
//         /// so it will effectively render a very zoomed out view of the scene in the center of the screen.
//         const GIZMO3 = 0b0000100;
//     }
// }
