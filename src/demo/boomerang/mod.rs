use crate::assets::BoomerangAssets;
use crate::demo::enemy::Enemy;
use crate::demo::health::{CanDamage, Health, HealthEvent};
use crate::demo::input::FireBoomerangAction;
use crate::demo::mouse_position::MousePosition;
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{
    Collider, CollisionEventsEnabled, CollisionLayers, RigidBody, SpatialQuery, SpatialQueryFilter,
};
use bevy::app::App;
use bevy::color;
use bevy::ecs::entity::EntityHashSet;
use bevy::math::Dir3;
use bevy::prelude::*;
use bevy::time::Time;
use bevy_enhanced_input::events::Fired;
use boomerang_settings::BoomerangSettings;
use log::error;

pub mod boomerang_settings;

pub const BOOMERANG_FLYING_HEIGHT: f32 = 0.5;

/// Component used to describe boomerang entities.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Boomerang {
    /// The path this boomerang is following.
    path: Vec<BoomerangTargetKind>,
    path_index: usize,
    progress_on_current_segment: f32, // value from 0.0 to 1.0
}
impl Boomerang {
    pub fn new(path: Vec<BoomerangTargetKind>) -> Self {
        Self {
            path,
            path_index: 0,
            progress_on_current_segment: 0.0,
        }
    }

    pub fn _is_last_segment(&self) -> bool {
        self.path_index >= self.path.len() - 2
    }
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
pub struct BounceBoomerangEvent {
    /// The boomerang entity
    pub boomerang_entity: Entity,
    /// The target we have bounced against
    pub _bounce_on: BoomerangTargetKind,
}

// An event which gets fired whenever a boomerang falls to the ground, thus ceasing all movement.
#[derive(Event)]
struct BoomerangHasFallenOnGroundEvent {
    /// The boomerang entity
    boomerang_entity: Entity,
}

/// An enum to differentiate between the different kinds of targets our boomerang may want to hit.
#[derive(Copy, Clone, Debug, PartialEq, Reflect)]
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

pub fn plugin(app: &mut App) {
    app.add_plugins(boomerang_settings::plugin);

    app.init_gizmo_group::<BoomerangPreviewGizmos>();
    app.add_event::<ThrowBoomerangEvent>();
    app.add_event::<BounceBoomerangEvent>();
    app.add_event::<BoomerangHasFallenOnGroundEvent>();
    app.init_resource::<BoomerangAssets>();

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
            (
                move_flying_boomerangs,
                on_boomerang_bounce_advance_to_next_pathing_step_or_fall_down,
                on_boomerang_collision,
            )
                .chain(),
            move_falling_boomerangs,
            on_boomerang_fallen_despawn_boomerang.after(move_falling_boomerangs),
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
    mut flying_boomerangs: Query<(Entity, &mut Boomerang, &mut Transform), With<Flying>>,
    all_other_transforms: Query<&Transform, Without<Boomerang>>,
    boomerang_settings: Res<BoomerangSettings>,
    time: Res<Time>,
    mut bounce_event_writer: EventWriter<BounceBoomerangEvent>,
) -> Result {
    for (boomerang_entity, mut boomerang, mut transform) in flying_boomerangs.iter_mut() {
        let target = &boomerang
            .path
            .get(boomerang.path_index + 1)
            .ok_or(format!("No path for boomerang {boomerang:?}"))?
            .clone();

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

        // todo make this a util fn
        let origin_position = match boomerang
            .path
            .get(boomerang.path_index)
            .ok_or(format!("No Origin for boomerang: {boomerang:?}"))?
        {
            BoomerangTargetKind::Entity(entity) => all_other_transforms
                .get(*entity)?
                .translation
                .with_y(BOOMERANG_FLYING_HEIGHT),
            BoomerangTargetKind::Position(position) => position.with_y(BOOMERANG_FLYING_HEIGHT),
        };

        let total_path_length = (target_position - origin_position).length();
        let progress = 1. - (remaining_distance / total_path_length);
        boomerang.progress_on_current_segment = progress; // convenience hack; cache this value so we don't have to recalculate in other systems.
        let velocity = boomerang_settings.tween_movement_speed(progress);

        let distance_travelled_this_frame = velocity * time.delta_secs();
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

fn on_boomerang_fallen_despawn_boomerang(
    mut fallen_events: EventReader<BoomerangHasFallenOnGroundEvent>,
    mut commands: Commands,
) -> Result {
    for event in fallen_events.read() {
        commands.entity(event.boomerang_entity).despawn();
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
    println!("Boomerang bounce event: {boomerang_entity}");
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

        boomerang.path_index += 1;

        if boomerang.path_index >= boomerang.path.len() - 1 {
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
    mut boomerangs: Query<(&mut Transform, &Boomerang), With<Flying>>,
    time: Res<Time>,
    settings: Res<BoomerangSettings>,
) {
    for (mut transform, boomerang) in boomerangs.iter_mut() {
        let rotation_speed = settings.tween_rotation_speed(boomerang.progress_on_current_segment);
        transform.rotate_local_y(rotation_speed * time.delta_secs());
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

    let max_distance = 10.0;
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
    _trigger: Trigger<Fired<FireBoomerangAction>>,
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
) -> Result {
    for event in event_reader.read() {
        // add the thrower as both the first and last node on the path
        let thrower = BoomerangTargetKind::Entity(event.thrower_entity);
        let mut path = vec![thrower];
        path.append(&mut event.target.clone());
        path.push(thrower);

        // spawn the 'rang
        commands.spawn((
            Name::new("Boomerang"),
            Transform::from_translation(
                all_transforms
                    .get(event.thrower_entity)?
                    .translation
                    .with_y(BOOMERANG_FLYING_HEIGHT),
            ),
            Boomerang::new(path),
            Flying,
            Mesh3d(boomerang_assets.mesh.clone()),
            MeshMaterial3d(boomerang_assets.material.clone()),
            Collider::sphere(0.5),
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy),
            RigidBody::Kinematic,
            CanDamage(1),
            CollisionEventsEnabled,
        ));
    }

    Ok(())
}

fn on_boomerang_collision(
    mut events: EventReader<BounceBoomerangEvent>,
    healths: Query<Entity, (With<Health>, With<Enemy>)>,
    mut commands: Commands,
) {
    for event in events.read() {
        println!("Boomerang Collision!");
        let BoomerangTargetKind::Entity(target) = event._bounce_on else {
            continue;
        };
        println!("Boomerang Collision on target {target:?}");
        if healths.contains(target) {
            commands.entity(target).trigger(HealthEvent::Damage(1));
            println!("Fired Health Damage Event");
        }
    }
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
