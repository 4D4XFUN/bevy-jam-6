use crate::assets::BoomerangAssets;
use crate::gameplay::mouse_position::MousePosition;
use crate::screens::Screen;
use bevy::app::App;
use bevy::color;
use bevy::input::ButtonInput;
use bevy::math::Dir3;
use bevy::prelude::{
    AppGizmoBuilder, Assets, BevyError, Color, Commands, Component, Cuboid, Entity, Event,
    EventReader, EventWriter, GizmoConfigGroup, Gizmos, GlobalTransform, Handle,
    IntoScheduleConfigs, Mesh, Mesh3d, MeshMaterial3d, MouseButton, Mut, Name, OnEnter, Plugin,
    Query, Reflect, Res, ResMut, Resource, StandardMaterial, Transform, Update, Vec3, With,
    Without, in_state, on_event,
};
use bevy::time::Time;
use log::error;
use std::collections::VecDeque;

const BOOMERANG_ROTATIONS_PER_SECOND: f32 = 6.0;
const BOOMERANG_FALL_SPEED: f32 = 1.0;

pub const BOOMERANG_FLYING_HEIGHT: f32 = 0.5;
const BOOMERANG_FLYING_OFFSET: Vec3 = Vec3::new(0.0, BOOMERANG_FLYING_HEIGHT, 0.0);

/// Component used to describe boomerang entities.
#[derive(Component, Default)]
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
struct BoomerangHittable;

/// Entities with this component will allow the user to redirect the boomerang bounce when they are hit by becoming an [ActiveBoomerangThrowOrigin]
#[derive(Component, Default)]
#[require(BoomerangHittable)]
struct PotentialBoomerangOrigin;

/// Component which should be added to the entity the boomerang is currently "attached" to.
/// Used to mark the origin for the next bounce direction.
#[derive(Component)]
#[require(PotentialBoomerangOrigin)]
struct ActiveBoomerangThrowOrigin;

pub(crate) struct BoomerangThrowingPlugin;
impl Plugin for BoomerangThrowingPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<BoomerangPreviewGizmos>();
        app.add_event::<ThrowBoomerangEvent>();
        app.add_event::<BounceBoomerangEvent>();
        app.add_event::<BoomerangHasFallenOnGroundEvent>();

        app.add_systems(
            Update,
            (
                (
                    update_boomerang_preview_position,
                    throw_boomerang_on_button_press,
                    (
                        draw_preview_gizmo,
                        on_throw_boomerang.run_if(on_event::<ThrowBoomerangEvent>),
                    ),
                )
                    .chain(),
                rotate_boomerangs,
                move_flying_boomerangs,
                on_boomerang_bounce.after(move_flying_boomerangs),
                move_falling_boomerangs,
                remove_falling_from_fallen_boomerangs.after(move_falling_boomerangs),
            )
                .run_if(in_state(Screen::Gameplay)),
        );

        // TODO: Remove this
        app.add_systems(OnEnter(Screen::Gameplay), spawn_test_entities);
    }
}

fn spawn_test_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(0.2, 1.0, 0.2));

    let player_material: Handle<StandardMaterial> = materials.add(Color::WHITE);
    let enemy_material: Handle<StandardMaterial> = materials.add(Color::linear_rgb(1.0, 0.2, 0.2));

    commands.spawn((
        Name::new("TestBoomerangThrower"),
        ActiveBoomerangThrowOrigin,
        Transform::from_translation(Vec3::ZERO),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(player_material.clone()),
    ));
    commands.spawn((
        Name::new("TestBoomerangTarget"),
        BoomerangHittable,
        PotentialBoomerangOrigin,
        Transform::from_translation(Vec3::new(5.0, 0.0, 2.0)),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(enemy_material.clone()),
    ));
}

/// Moves boomerangs along their paths.
/// Fires a [BounceBoomerangEvent] in case that the next path destination was reached.
fn move_flying_boomerangs(
    mut flying_boomerangs: Query<(Entity, &Boomerang, &mut Transform), With<Flying>>,
    all_other_transforms: Query<&Transform, Without<Boomerang>>,
    time: Res<Time>,
    mut bounce_event_writer: EventWriter<BounceBoomerangEvent>,
) -> Result<(), BevyError> {
    for (boomerang_entity, boomerang, mut transform) in flying_boomerangs.iter_mut() {
        let Some(target) = boomerang.path.front() else {
            panic!("Boomerang path list was empty?")
        };

        let target_position = match target {
            BoomerangTargetKind::Entity(entity) => &all_other_transforms.get(*entity)?.translation,
            BoomerangTargetKind::Position(position) => position,
        };

        let Ok((direction, remaining_distance)) =
            Dir3::new_and_length(target_position - transform.translation)
        else {
            send_boomerang_bounce_event(
                &mut bounce_event_writer,
                boomerang_entity,
                &mut transform,
                target,
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
                target,
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
) -> Result<(), BevyError> {
    for (entity, mut transform) in falling_boomerangs.iter_mut() {
        transform.translation.y -= BOOMERANG_FALL_SPEED * time.delta_secs();

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

fn remove_falling_from_fallen_boomerangs(
    mut bounce_events: EventReader<BoomerangHasFallenOnGroundEvent>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    for event in bounce_events.read() {
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
    target: &BoomerangTargetKind,
    target_position: &Vec3,
) {
    transform.translation = target_position.clone();
    bounce_event_writer.write(BounceBoomerangEvent {
        boomerang_entity,
        bounce_on: target.clone(),
    });
}

fn on_boomerang_bounce(
    mut bounce_events: EventReader<BounceBoomerangEvent>,
    mut boomerangs: Query<&mut Boomerang, With<Flying>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
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
) {
    for mut transform in boomerangs.iter_mut() {
        transform.rotate_local_y(BOOMERANG_ROTATIONS_PER_SECOND * time.delta_secs());
    }
}

// An event which gets fired whenever the player throws their boomerang.
#[derive(Event)]
struct ThrowBoomerangEvent {
    origin: Entity,
    target: BoomerangTargetKind,
}

// An event which gets fired whenever a boomerang reaches the end of its current path.
#[derive(Event)]
struct BounceBoomerangEvent {
    /// The boomerang entity
    boomerang_entity: Entity,
    /// The target we have bounced against
    bounce_on: BoomerangTargetKind,
}

// An event which gets fired whenever a boomerang falls to the ground, thus ceasing all movement.
#[derive(Event)]
struct BoomerangHasFallenOnGroundEvent {
    /// The boomerang entity
    boomerang_entity: Entity,
}

/// An enum to differentiate between the different kinds of targets our boomerang may want to hit.
#[derive(Copy, Clone)]
enum BoomerangTargetKind {
    /// Targeting an entity means it will home in on it, even as it moves.
    Entity(Entity),
    /// Targeting a position means the boomerang will always fly in a straight line there.
    Position(Vec3),
}

/// The path our boomerang is supposed to follow.
#[derive(Resource)]
struct PlannedBoomerangPath {
    targets: Vec<BoomerangTargetKind>,
}

/// Component for the preview entity for the next boomerang target location.
#[derive(Component)]
struct BoomerangPathPreview {
    /// The entity that's being targeted, if there is any.
    entity: Option<Entity>,
}

fn update_boomerang_preview_position(
    boomerang_origins: Query<&GlobalTransform, With<ActiveBoomerangThrowOrigin>>,
    boomerang_targets: Query<(Entity, &GlobalTransform), With<BoomerangHittable>>,
    mut previews: Query<(&mut BoomerangPathPreview, &mut Transform)>,
    mouse_position: Res<MousePosition>,
    mut commands: Commands,
) {
    let Some(mouse_position) = mouse_position.boomerang_throwing_plane else {
        // Mouse is probably not inside the game window right now
        return;
    };

    let Ok(origin_transform) = boomerang_origins.single() else {
        error!("There was no boomerang origin to update the preview?");
        return;
    };

    let Ok(direction) = Dir3::new(mouse_position - origin_transform.translation()) else {
        // We are probably just pointing right at the ThrowOrigin
        return;
    };

    // TODO: Raycast to see what and where we hit something.
    let preview_location = direction * 10.0;
    let target_entity = None;

    if let Ok((mut preview, mut transform)) = previews.single_mut() {
        preview.entity = target_entity;
        transform.translation = preview_location;
    } else {
        commands.spawn((
            BoomerangPathPreview {
                entity: target_entity,
            },
            Transform::from_translation(preview_location),
        ));
    }
}

// TODO: use bevy_enhanced_input for the button press
fn throw_boomerang_on_button_press(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    boomerang_holders: Query<Entity, With<ActiveBoomerangThrowOrigin>>,
    boomerang_previews: Query<(&BoomerangPathPreview, &GlobalTransform)>,
    mut event_writer: EventWriter<ThrowBoomerangEvent>,
) {
    if !mouse_buttons.just_released(MouseButton::Left) {
        return;
    }

    let Ok(thrower_entity) = boomerang_holders.single() else {
        error!("Was unable to find a single thrower! (multiple ain't supported yet)");
        return;
    };
    let Ok((preview, preview_position)) = boomerang_previews.single() else {
        error!("Was unable to find a single target preview! (multiple ain't supported yet)");
        return;
    };

    let target = match preview.entity {
        None => BoomerangTargetKind::Position(preview_position.translation()),
        Some(entity) => BoomerangTargetKind::Entity(entity),
    };

    event_writer.write(ThrowBoomerangEvent {
        origin: thrower_entity,
        target,
    });
}

fn on_throw_boomerang(
    mut event_reader: EventReader<ThrowBoomerangEvent>,
    mut commands: Commands,
    all_transforms: Query<&Transform>,
    boomerang_assets: Res<BoomerangAssets>,
) -> Result<(), BevyError> {
    for event in event_reader.read() {
        commands.spawn((
            Name::new("Boomerang"),
            Transform::from_translation(
                all_transforms.get(event.origin)?.translation + BOOMERANG_FLYING_OFFSET,
            ),
            Boomerang {
                path: vec![event.target].into(),
                speed: 10.0,
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
                from.translation() + BOOMERANG_FLYING_OFFSET,
                to.translation(),
                color::palettes::css::ORANGE,
            );
        }
    }
}
