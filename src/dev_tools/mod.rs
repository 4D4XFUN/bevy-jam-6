//! Development tools for the game. This plugin is only enabled in dev builds.

mod god_mode;

use crate::screens::Screen;
use avian3d::prelude::PhysicsGizmos;
use bevy::audio::Volume;
use bevy::color::palettes;
use bevy::dev_tools::states::log_transitions;
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use iyes_perf_ui::PerfUiPlugin;
use iyes_perf_ui::entries::{PerfUiFramerateEntries, PerfUiWindowEntries};
use iyes_perf_ui::prelude::{PerfUiPosition, PerfUiRoot};
use crate::dev_tools::god_mode::GodModeState;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        PerfUiPlugin,
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        bevy::diagnostic::EntityCountDiagnosticsPlugin,
        // https://github.com/IyesGames/iyes_perf_ui/issues/30
        // bevy::diagnostic::SystemInformationDiagnosticsPlugin,
        bevy::render::diagnostic::RenderDiagnosticsPlugin,
        avian3d::debug_render::PhysicsDebugPlugin::new(FixedUpdate),

        // inspector
        EguiPlugin { enable_multipass_for_primary_context: true, },
        WorldInspectorPlugin::new().run_if(in_state(GodModeState::God)),
        // boomerang_dev_tools_plugin,

        #[cfg(feature = "dev")]
        god_mode::plugin,
    ))
    .insert_gizmo_config(
        PhysicsGizmos {
            shapecast_color: Some(palettes::css::AQUAMARINE.into()),
            raycast_color: Some(palettes::css::BLUE_VIOLET.into()),
            ..default()
        },
        GizmoConfig::default(),
    );

    // Log `Screen` state transitions.
    app.add_systems(Update, log_transitions::<Screen>);

    app.add_systems(Startup, (setup_perf_ui, lower_starting_audio_volume));
}

#[derive(Component)]
pub struct PerfUiMarker;

fn lower_starting_audio_volume(mut global_volume: ResMut<GlobalVolume>) {
    global_volume.volume = Volume::Linear(0.5);
}

fn setup_perf_ui(mut commands: Commands) {
    commands.spawn((
        Name::from("PerfUi"),
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
