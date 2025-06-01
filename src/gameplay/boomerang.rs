use crate::gameplay::mouse_position::MousePosition;
use bevy::app::App;
use bevy::color;
use bevy::math::Dir3;
use bevy::prelude::{
    AppGizmoBuilder, Color, Commands, Component, Entity, Event, EventReader, EventWriter,
    GizmoConfigGroup, Gizmos, GlobalTransform, IntoScheduleConfigs, MouseButton, Name, Plugin,
    PostUpdate, Query, Reflect, Res, Resource, Srgba, Startup, Transform, Trigger, Update, Vec3,
    With, in_state,
};
use bevy_enhanced_input::actions::Actions;
use bevy_enhanced_input::prelude::{Fired, InputAction, InputContext, Release};
use log::error;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct ThrowBoomerang;

#[derive(InputContext)]
struct MyInputContext;

/// Component used to mark actively flying boomerangs.
#[derive(Component, Default)]
struct Boomerang {
    /// The path this boomerang is following.
    path: Vec<BoomerangTargetKind>,
}

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
        let mut actions = Actions::<MyInputContext>::default();
        actions
            .bind::<ThrowBoomerang>()
            .to((MouseButton::Left))
            .with_conditions(Release::default());
        app.init_gizmo_group::<BoomerangPreviewGizmos>();
        app.add_event::<ThrowBoomerangEvent>();
        app.add_observer(throw_boomerang_on_button_press);

        app.add_systems(
            Update,
            (
                update_boomerang_preview_position,
                (draw_preview_gizmo, on_throw_boomerang),
            )
                .chain(),
        );

        // TODO: Remove
        app.add_systems(Startup, spawn_test_entities);
    }
}

fn spawn_test_entities(mut commands: Commands) {
    commands.spawn((
        Name::new("TestBoomerangThrower"),
        ActiveBoomerangThrowOrigin,
        Transform::from_translation(Vec3::ZERO),
    ));
    commands.spawn((
        Name::new("TestBoomerangTarget"),
        BoomerangHittable,
        PotentialBoomerangOrigin,
        Transform::from_translation(Vec3::X * 10.0),
    ));
}

// An event which gets fired whenever the player throws their boomerang.
#[derive(Event)]
struct ThrowBoomerangEvent {
    origin: Entity,
    target: BoomerangTargetKind,
}

/// An enum to differentiate between the different kinds of targets our boomerang may want to hit.
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
    let Some(global_mouse_position) = mouse_position.global else {
        // Mouse is probably not inside the game window right now
        return;
    };

    let Ok(origin_transform) = boomerang_origins.single() else {
        error!("There was no boomerang origin to update the preview?");
        return;
    };

    let Ok(direction) = Dir3::new(global_mouse_position - origin_transform.translation()) else {
        // We are probably just pointing right at the ThrowOrigin
        return;
    };

    // TODO: Raycast to see what and where we hit something
    let preview_location = direction * 100.0;
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

fn throw_boomerang_on_button_press(
    trigger: Trigger<Fired<ThrowBoomerang>>,
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

    let target = match preview.entity {
        None => BoomerangTargetKind::Position(preview_position.translation()),
        Some(entity) => BoomerangTargetKind::Entity(entity),
    };

    event_writer.write(ThrowBoomerangEvent {
        origin: thrower_entity,
        target,
    });
}

fn on_throw_boomerang(mut event_reader: EventReader<ThrowBoomerangEvent>, mut commands: Commands) {
    for event in event_reader.read() {}
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
                from.translation(),
                to.translation(),
                color::palettes::css::ORANGE,
            );
        }
    }
}
