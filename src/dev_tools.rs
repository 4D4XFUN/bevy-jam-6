//! Development tools for the game. This plugin is only enabled in dev builds.

use bevy::dev_tools::states::log_transitions;
use bevy::prelude::*;
use iyes_perf_ui::entries::{PerfUiFramerateEntries, PerfUiWindowEntries};
use iyes_perf_ui::PerfUiPlugin;
use iyes_perf_ui::prelude::{PerfUiPosition, PerfUiRoot};
use crate::screens::Screen;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        PerfUiPlugin,
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        bevy::diagnostic::EntityCountDiagnosticsPlugin,
        // https://github.com/IyesGames/iyes_perf_ui/issues/30
        // bevy::diagnostic::SystemInformationDiagnosticsPlugin,
        bevy::render::diagnostic::RenderDiagnosticsPlugin,
    ));

    // Log `Screen` state transitions.
    app.add_systems(Update, log_transitions::<Screen>);

    app.add_systems(Startup, setup_perf_ui);
}

#[derive(Component)]
pub struct PerfUiMarker;

fn setup_perf_ui(mut commands: Commands) {
    commands.spawn((
        PerfUiMarker,
        PerfUiRoot {
            position: PerfUiPosition::TopRight,
            ..default()
        },
        // Contains everything related to FPS and frame time
        PerfUiFramerateEntries::default(),
        // Contains everything related to the window and cursor
        PerfUiWindowEntries::default(),
        // Contains everything related to system diagnostics (CPU, RAM)
        // PerfUiSystemEntries::default(),
    ));
}