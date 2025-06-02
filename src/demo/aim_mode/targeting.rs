use bevy::prelude::*;
use crate::demo::aim_mode::AimModeState;
use crate::demo::mouse_position::MousePosition;

/// While in aim mode, this module handles queueing up a list of targets,
/// displaying crosshairs, and creating the target list for the boomerang to
/// follow once we exit aim mode.
pub fn plugin(app: &mut App) {
    app.add_systems(Update, draw_crosshair.run_if(in_state(AimModeState::Aiming)));
}

#[derive(Component, Default, Debug, Clone)]
struct AimModeTargets {
    targets: Vec<TargetableThing>,
    // todo add each "painted" target to this list as we mouse over them in aim mode
    // todo snap crosshairs to each target to give player feedback about what they're going to hit
    // todo when aim mode exits, despawn this entity and fire a single boomerang with the list of targets we painted
}

fn draw_crosshair(mut gizmos: Gizmos, mouse_position: Res<MousePosition>) {
    let Some(mouse_position) = mouse_position.boomerang_throwing_plane else {
        debug!("No mouse position found");
        return;
    };

    // Create a rotation that rotates 90 degrees (PI/2 radians) around the X-axis
    let rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let isometry = Isometry3d::new(mouse_position, rotation);

    gizmos.circle(isometry, 2.0, Color::srgb(0.9, 0.1, 0.1));
}
