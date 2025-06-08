// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]
// clippy inconsistencies
#![allow(clippy::collapsible_if)]

mod ai;
mod asset_tracking;
mod audio;
#[cfg(feature = "dev")]
mod dev_tools;
mod framepace;
mod gameplay;
mod physics_layers;
mod screens;
mod theme;
mod ui_assets;

use avian3d::PhysicsPlugins;
use bevy::window::{PresentMode, WindowResolution};
use bevy::{asset::AssetMetaCheck, prelude::*};
use bevy_hanabi::HanabiPlugin;
use bevy_skein::SkeinPlugin;
use oxidized_navigation::OxidizedNavigation;

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
                AppSystems::PreTickTimers,
                AppSystems::TickTimers,
                AppSystems::RecordInput,
                AppSystems::Update,
            )
                .chain(),
        );

        app.configure_sets(
            RunFixedMainLoop,
            (
                OxidizedNavigation::RemovedComponent,
                OxidizedNavigation::Main,
            )
                .chain()
                .in_set(RunFixedMainLoopSystem::BeforeFixedMainLoop),
        );

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
                        title: "FISTFUL OF BOOMERANGS".to_string(),
                        present_mode: PresentMode::AutoNoVsync,
                        fit_canvas_to_parent: true,
                        resolution: WindowResolution::new(1024., 768.),
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            SkeinPlugin::default(),
            PhysicsPlugins::default(),
            HanabiPlugin,
        ));

        // Add other plugins.
        app.add_plugins((
            asset_tracking::plugin,
            #[cfg(feature = "dev")]
            dev_tools::plugin,
            screens::plugin,
            theme::plugin,
            framepace::plugin,
            gameplay::plugin,
            ai::plugin,
        ));

        app.add_systems(Update, audio::update_sfx_speed);
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
/// When adding a new variant, make sure to order it in the `configure_sets`
/// call above.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum AppSystems {
    /// Happens before timers tick, for time-manipulation stuff that subsequent timers need
    PreTickTimers,
    /// Tick timers.
    TickTimers,
    /// Record player input.
    RecordInput,
    /// Do everything else (consider splitting this into further variants).
    Update,
}
