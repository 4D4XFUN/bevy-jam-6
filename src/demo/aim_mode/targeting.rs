use crate::demo::aim_mode::AimModeState;
use crate::demo::boomerang::{BoomerangHittable, BoomerangTargetKind};
use crate::demo::enemy::Enemy;
use crate::demo::mouse_position::MousePosition;
use crate::screens::Screen;
use avian3d::prelude::*;
use bevy::ecs::error::info;
use bevy::prelude::*;

/// While in aim mode, this module handles queueing up a list of targets,
/// displaying crosshairs, and creating the target list for the boomerang to
/// follow once we exit aim mode.
pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (draw_crosshair, draw_target_circles).run_if(in_state(AimModeState::Aiming)),
    );

    app.add_systems(Update, record_target_near_mouse);

    app.add_systems(OnEnter(AimModeState::Aiming), initialize_target_list);
    app.add_systems(OnExit(AimModeState::Aiming), cleanup_target_list);
}

const AUTOTARGETING_RADIUS: f32 = 2.0;

#[derive(Component, Default, Debug, Clone)]
struct AimModeTargets {
    targets: Vec<Entity>,
    // todo add each "painted" target to this list as we mouse over them in aim mode
    // todo snap crosshairs to each target to give player feedback about what they're going to hit
    // todo when aim mode exits, despawn this entity and fire a single boomerang with the list of targets we painted
}

fn initialize_target_list(mut commands: Commands) {
    commands.spawn(AimModeTargets::default());
}

fn cleanup_target_list(mut commands: Commands, mut query: Single<(Entity, &AimModeTargets)>) {
    let (e, targets) = query.into_inner();
    info!(
        "Cleaning up target list: {:?}",
        targets.targets
    );

    commands.entity(e).despawn();
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

fn draw_target_circles(mut gizmos: Gizmos,
                       hittables: Query<&Transform, With<BoomerangHittable>>,
                       query: Single<&AimModeTargets>) {

    let targets = query.into_inner();
    let x = &targets.targets;

    for e in x.iter() {
        if let Some(t) = hittables.get(*e).ok() {
            // Create a rotation that rotates 90 degrees (PI/2 radians) around the X-axis
            let rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
            let isometry = Isometry3d::new(t.translation, rotation);

            // todo use retained mode gizmos to be more efficient (or an instanced mesh of a cool looking crosshair)
            gizmos.circle(isometry, 1.5, Color::srgb(0.9, 0.1, 0.1));
        }
    }

}

// some reasonable max that should never be reached during real gameplay (once we implement boomerang energy)
const MAX_TARGETS_SELECTABLE: usize = 10;

fn record_target_near_mouse(
    mouse_position: Res<MousePosition>,
    spatial_query: SpatialQuery,
    potential_targets: Query<Entity, With<BoomerangHittable>>,
    mut current_target_list: Single<&mut AimModeTargets>,
) -> Result {
    // target list is full, don't add any more targets
    if current_target_list.targets.len() > MAX_TARGETS_SELECTABLE {
        return Ok(());
    }

    let Some(mouse_position) = mouse_position.boomerang_throwing_plane else {
        warn!("No mouse position found");
        return Ok(());
    };

    let direction = Dir3::X;
    let config = ShapeCastConfig::from_max_distance(100.0);
    let filter = SpatialQueryFilter::default();
    let Some(hit) = spatial_query.cast_shape(
        &Collider::sphere(AUTOTARGETING_RADIUS), // Shape
        mouse_position,                          // Shape position
        Quat::default(),                         // Shape rotation
        direction,
        &config,
        &filter,
    ) else {
        return Ok(());
    };

    let last_target = current_target_list.targets.last();

    match last_target {
        Some(&e) if e == hit.entity => {
            return Ok(());
        }
        _ => {
            current_target_list.targets.push(hit.entity);
            info!(
                "Adding target to list {:?}. List after addition: {:?}",
                hit.entity, &current_target_list.targets
            );
        }
    }

    Ok(())
}
