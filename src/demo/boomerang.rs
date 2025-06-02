use crate::assets::BoomerangAssets;
use crate::demo::mouse_position::MousePosition;
use crate::screens::Screen;
use avian3d::prelude::{Collider, SpatialQuery, SpatialQueryFilter};
use bevy::app::App;
use bevy::color;
use bevy::ecs::entity::EntityHashSet;
use bevy::input::ButtonInput;
use bevy::math::Dir3;
use bevy::prelude::*;
use bevy::time::Time;
use log::{error, warn};
use std::collections::VecDeque;
use bevy_enhanced_input::events::Fired;
use crate::demo::input::FireBoomerangAction;

pub const BOOMERANG_FLYING_HEIGHT: f32 = 0.5;

/// Component used to describe boomerang entities.
#[derive(Component, Debug, Default)]
struct Boomerang {
    /// The path this boomerang is following.
    path: VecDeque<BoomerangTargetKind>,
    speed: f32,
}

/// Component used to mark boomerangs which are midair.
#[derive(Component)]
struct Flying;

/// Component used to mark boomerangs which have reached their final location and are now falling.
#[derive(Component)]
struct Falling;

/// Component used to mark anything which can be hit by the boomerang.
/// By default, the Boomerang will just bounce off of the marked surface (like a wall), add other components like [PotentialBoomerangOrigin] to add more functionality.
#[derive(Component, Default)]
pub struct BoomerangHittable;

/// Entities with this component will allow the user to redirect the boomerang bounce when they are hit by becoming an [ActiveBoomerangThrowOrigin]
#[derive(Component, Default)]
#[require(BoomerangHittable)]
pub struct PotentialBoomerangOrigin;

/// Component which should be added to the entity the boomerang is currently "attached" to.
/// Used to mark the origin for the next bounce direction.
#[derive(Component)]
#[require(PotentialBoomerangOrigin)]
pub struct ActiveBoomerangThrowOrigin;

// An event which gets fired whenever the player throws their boomerang.
#[derive(Event)]
pub struct ThrowBoomerangEvent {
    pub thrower_entity: Entity,
    pub target: Vec<BoomerangTargetKind>,
}

// An event which gets fired whenever a boomerang reaches the end of its current path.
#[derive(Event)]
struct BounceBoomerangEvent {
    /// The boomerang entity
    boomerang_entity: Entity,
    /// The target we have bounced against
    _bounce_on: BoomerangTargetKind,
}

// An event which gets fired whenever a boomerang falls to the ground, thus ceasing all movement.
#[derive(Event)]
struct BoomerangHasFallenOnGroundEvent {
    /// The boomerang entity
    boomerang_entity: Entity,
}

/// An enum to differentiate between the different kinds of targets our boomerang may want to hit.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BoomerangTargetKind {
    /// Targeting an entity means it will home in on it, even as it moves.
    Entity(Entity),
    /// Targeting a position means the boomerang will always fly in a straight line there.
    Position(Vec3),
}

/// Component for the preview entity for the next boomerang target location.
#[derive(Component)]
struct BoomerangPathPreview {
    /// The entity that's being targeted, if there is any.
    target_entity: Option<Entity>,
}

/// Current set of stats of our boomerang
#[derive(Resource)]
struct BoomerangSettings {
    movement_speed: f32,
    rotations_per_second: f32,
    falling_speed: f32,
}

impl Default for BoomerangSettings {
    fn default() -> Self {
        Self {
            movement_speed: 20.0,
            rotations_per_second: 12.0,
            falling_speed: 4.0,
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_gizmo_group::<BoomerangPreviewGizmos>();
    app.add_event::<ThrowBoomerangEvent>();
    app.add_event::<BounceBoomerangEvent>();
    app.add_event::<BoomerangHasFallenOnGroundEvent>();
    app.init_resource::<BoomerangAssets>();
    app.init_resource::<BoomerangSettings>();

    app.add_systems(
        Update,
        (
            (
                update_boomerang_preview_position,
                (
                    draw_preview_gizmo,
                    on_throw_boomerang_spawn_boomerang.run_if(on_event::<ThrowBoomerangEvent>),
                ),
            )
                .chain(),
            rotate_boomerangs,
            move_flying_boomerangs,
            on_boomerang_bounce_advance_to_next_pathing_step_or_fall_down
                .after(move_flying_boomerangs),
            move_falling_boomerangs,
            on_boomerang_fallen_remove_falling_component.after(move_falling_boomerangs),
        )
            .run_if(in_state(Screen::Gameplay)),
    );

    app.add_observer(on_fire_action_throw_boomerang);

    // TODO: Remove this
    app.add_systems(OnEnter(Screen::Gameplay), spawn_test_entities);
}

fn spawn_test_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(0.5, 1.5, 0.5));
    let enemy_material: Handle<StandardMaterial> = materials.add(Color::linear_rgb(1.0, 0.2, 0.2));

    commands.spawn((
        Name::new("TestBoomerangTarget"),
        BoomerangHittable,
        PotentialBoomerangOrigin,
        Transform::from_translation(Vec3::new(5.0, 0.0, 2.0)),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(enemy_material.clone()),
        Collider::cuboid(0.5, 1.5, 0.5),
    ));
}

/// Moves boomerangs along their paths.
/// Fires a [BounceBoomerangEvent] in case that the next path destination was reached.
fn move_flying_boomerangs(
    mut flying_boomerangs: Query<(Entity, &Boomerang, &mut Transform), With<Flying>>,
    all_other_transforms: Query<&Transform, Without<Boomerang>>,
    time: Res<Time>,
    mut bounce_event_writer: EventWriter<BounceBoomerangEvent>,
) -> Result {
    for (boomerang_entity, boomerang, mut transform) in flying_boomerangs.iter_mut() {
        let target = boomerang
            .path
            .front()
            .ok_or(format!("No path for boomerang {boomerang:?}"))?;

        let target_position = match target {
            BoomerangTargetKind::Entity(entity) => all_other_transforms
                .get(*entity)?
                .translation
                .with_y(BOOMERANG_FLYING_HEIGHT),
            BoomerangTargetKind::Position(position) => position.with_y(BOOMERANG_FLYING_HEIGHT),
        };

        let Ok((direction, remaining_distance)) = Dir3::new_and_length(
            target_position - transform.translation.with_y(BOOMERANG_FLYING_HEIGHT),
        ) else {
            send_boomerang_bounce_event(
                &mut bounce_event_writer,
                boomerang_entity,
                &mut transform,
                *target,
                target_position,
            );
            continue;
        };

        let distance_travelled_this_frame = boomerang.speed * time.delta_secs();
        if remaining_distance <= distance_travelled_this_frame {
            send_boomerang_bounce_event(
                &mut bounce_event_writer,
                boomerang_entity,
                &mut transform,
                *target,
                target_position,
            );
            continue;
        }

        transform.translation += direction * distance_travelled_this_frame;
    }

    Ok(())
}

/// Lets boomerangs fall to the ground.
/// Fires a [BoomerangHasFallenOnGroundEvent] in case that the next path destination was reached.
fn move_falling_boomerangs(
    mut falling_boomerangs: Query<(Entity, &mut Transform), (With<Boomerang>, With<Falling>)>,
    time: Res<Time>,
    mut fallen_event_writer: EventWriter<BoomerangHasFallenOnGroundEvent>,
    boomerang_stats: Res<BoomerangSettings>,
) -> Result {
    for (entity, mut transform) in falling_boomerangs.iter_mut() {
        transform.translation.y -= boomerang_stats.falling_speed * time.delta_secs();

        // Probably needs to be raised a bit once we got a proper boomerang mesh
        if transform.translation.y <= 0.0 {
            transform.translation.y = 0.0;
            fallen_event_writer.write(BoomerangHasFallenOnGroundEvent {
                boomerang_entity: entity,
            });
        }
    }

    Ok(())
}

fn on_boomerang_fallen_remove_falling_component(
    mut fallen_events: EventReader<BoomerangHasFallenOnGroundEvent>,
    mut commands: Commands,
) -> Result {
    for event in fallen_events.read() {
        commands
            .entity(event.boomerang_entity)
            .remove::<Falling>()
            .insert(Falling);
    }

    Ok(())
}

fn send_boomerang_bounce_event(
    bounce_event_writer: &mut EventWriter<BounceBoomerangEvent>,
    boomerang_entity: Entity,
    transform: &mut Mut<Transform>,
    target: BoomerangTargetKind,
    target_position: Vec3,
) {
    transform.translation = target_position;
    bounce_event_writer.write(BounceBoomerangEvent {
        boomerang_entity,
        _bounce_on: target,
    });
}

fn on_boomerang_bounce_advance_to_next_pathing_step_or_fall_down(
    mut bounce_events: EventReader<BounceBoomerangEvent>,
    mut boomerangs: Query<&mut Boomerang, With<Flying>>,
    mut commands: Commands,
) -> Result {
    for event in bounce_events.read() {
        let mut boomerang = boomerangs.get_mut(event.boomerang_entity)?;

        boomerang.path.pop_front();

        if boomerang.path.is_empty() {
            commands
                .entity(event.boomerang_entity)
                .remove::<Flying>()
                .insert(Falling);
        }
    }

    Ok(())
}

/// Rotates our boomerangs at constant speed.
fn rotate_boomerangs(
    mut boomerangs: Query<&mut Transform, (With<Boomerang>, With<Flying>)>,
    time: Res<Time>,
    boomerang_stats: Res<BoomerangSettings>,
) {
    for mut transform in boomerangs.iter_mut() {
        transform.rotate_local_y(boomerang_stats.rotations_per_second * time.delta_secs());
    }
}

fn update_boomerang_preview_position(
    boomerang_origins: Single<(Entity, &GlobalTransform), With<ActiveBoomerangThrowOrigin>>,
    potential_origins: Query<(), With<PotentialBoomerangOrigin>>,
    mut previews: Query<(&mut BoomerangPathPreview, &mut Transform)>,
    mouse_position: Res<MousePosition>,
    mut commands: Commands,
    spatial_query: SpatialQuery,
) -> Result {
    let Some(mouse_position) = mouse_position.boomerang_throwing_plane else {
        // Mouse is probably not inside the game window right now
        return Ok(());
    };

    let (origin_entity, origin_transform) = boomerang_origins.into_inner();

    let origin = origin_transform
        .translation()
        .with_y(BOOMERANG_FLYING_HEIGHT);

    let Ok(direction) = Dir3::new(mouse_position - origin) else {
        // We are probably just pointing right at the ThrowOrigin
        return Ok(());
    };

    let max_distance = 100.0;
    let solid = true;
    let filter = SpatialQueryFilter {
        excluded_entities: EntityHashSet::from([origin_entity]),
        ..Default::default()
    };
    let (distance_to_target, target_entity) = if let Some(first_hit) =
        spatial_query.cast_ray(origin, direction, max_distance, solid, &filter)
    {
        if potential_origins.get(first_hit.entity).is_ok() {
            // It's something that can be used as an origin, so we want to home at it!
            // ...might want to adjust the filter in that query if we ever need to home in on non-boomerang-origins.
            (first_hit.distance, Some(first_hit.entity))
        } else {
            // It's a wall.
            (first_hit.distance, None)
        }
    } else {
        // warn!(
        //     "Unable to find a raycast target? Maybe we aren't in an enclosed room right now? If that's ever wanted, we probably need to also set up some max flying distance"
        // );

        (max_distance, None)
    };

    let target_location = origin + direction * distance_to_target;

    if let Ok((mut preview, mut transform)) = previews.single_mut() {
        preview.target_entity = target_entity;
        transform.translation = target_location;
    } else {
        // TODO: Preview needs to be despawned after throw
        commands.spawn((
            BoomerangPathPreview { target_entity },
            Transform::from_translation(target_location),
        ));
    }
    Ok(())
}

fn on_fire_action_throw_boomerang(
    trigger: Trigger<Fired<FireBoomerangAction>>,
    boomerang_holders: Query<Entity, With<ActiveBoomerangThrowOrigin>>,
    boomerang_previews: Query<(&BoomerangPathPreview, &GlobalTransform)>,
    mut event_writer: EventWriter<ThrowBoomerangEvent>,
) {
    let Ok(thrower_entity) = boomerang_holders.single() else {
        error!("Was unable to find a single thrower! (multiple ain't supported yet)");
        return;
    };
    let Ok((preview, preview_position)) = boomerang_previews.single() else {
        error!("Was unable to find a single target preview! (multiple ain't supported yet)");
        return;
    };

    let target = match preview.target_entity {
        None => BoomerangTargetKind::Position(preview_position.translation()),
        Some(entity) => BoomerangTargetKind::Entity(entity),
    };

    event_writer.write(ThrowBoomerangEvent {
        thrower_entity,
        target: vec![target],
    });   
}

fn on_throw_boomerang_spawn_boomerang(
    mut event_reader: EventReader<ThrowBoomerangEvent>,
    mut commands: Commands,
    all_transforms: Query<&Transform>,
    boomerang_assets: Res<BoomerangAssets>,
    boomerang_stats: Res<BoomerangSettings>,
) -> Result {
    for event in event_reader.read() {
        
        // add player as the last node on the path
        let mut path = event.target.clone();
        path.push(BoomerangTargetKind::Entity(event.thrower_entity));
        let path = VecDeque::from(path);
        
        // spawn the 'rang
        commands.spawn((
            Name::new("Boomerang"),
            Transform::from_translation(
                all_transforms
                    .get(event.thrower_entity)?
                    .translation
                    .with_y(BOOMERANG_FLYING_HEIGHT),
            ),
            Boomerang {
                path,
                speed: boomerang_stats.movement_speed,
            },
            Flying,
            Mesh3d(boomerang_assets.mesh.clone()),
            MeshMaterial3d(boomerang_assets.material.clone()),
        ));
    }

    Ok(())
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct BoomerangPreviewGizmos;

fn draw_preview_gizmo(
    mut gizmos: Gizmos<BoomerangPreviewGizmos>,
    boomerang_holders: Query<&GlobalTransform, With<ActiveBoomerangThrowOrigin>>,
    boomerang_target_preview: Query<&GlobalTransform, With<BoomerangPathPreview>>,
) {
    for from in boomerang_holders {
        for to in boomerang_target_preview {
            gizmos.line(
                from.translation().with_y(BOOMERANG_FLYING_HEIGHT),
                to.translation().with_y(BOOMERANG_FLYING_HEIGHT),
                color::palettes::css::ORANGE,
            );
        }
    }
}
