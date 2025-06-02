use crate::demo::aim_mode::AimModeState;
use crate::demo::boomerang::{BoomerangHittable, BoomerangTargetKind, ThrowBoomerangEvent};
use crate::demo::mouse_position::MousePosition;
use crate::demo::player::Player;
use crate::physics_layers::GameLayer;
use avian3d::prelude::*;
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
    // todo when aim mode exits, despawn this entity and fire a single boomerang with the list of targets we painted
}

fn initialize_target_list(mut commands: Commands) {
    commands.spawn(AimModeTargets::default());
}

fn cleanup_target_list(
    mut commands: Commands,
    query: Single<(Entity, &AimModeTargets)>,
    player_single: Single<Entity, With<Player>>,
    mut event_writer: EventWriter<ThrowBoomerangEvent>,
) {
    let (e, targets) = query.into_inner();
    let v: Vec<_> = targets
        .targets
        .iter()
        .map(|e| BoomerangTargetKind::Entity(*e))
        .collect();
    let player = player_single.into_inner(); // todo not why we nee this or how to handle multiple such entities. just assuming throws always originate from the player for now.

    event_writer.write(ThrowBoomerangEvent {
        thrower_entity: player,
        target: v,
    });
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

fn draw_target_circles(
    mut gizmos: Gizmos,
    hittables: Query<&Transform, With<BoomerangHittable>>,
    query: Single<&AimModeTargets>,
) {
    let targets = query.into_inner();
    let x = &targets.targets;

    for e in x.iter() {
        if let Ok(t) = hittables.get(*e) {
            // Create a rotation that rotates 90 degrees (PI/2 radians) around the X-axis
            let rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
            let isometry = Isometry3d::new(t.translation, rotation);

            // todo use retained mode gizmos to be more efficient (or an instanced mesh of a cool looking crosshair)
            gizmos.circle(isometry, 1.5, Color::srgb(0.9, 0.1, 0.1));
        }
    }
    // todo draw a line from player to first target, first target to second, etc.
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
    let config = ShapeCastConfig::from_max_distance(0.0);
    let filter = SpatialQueryFilter::from_mask(GameLayer::Enemy);
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
