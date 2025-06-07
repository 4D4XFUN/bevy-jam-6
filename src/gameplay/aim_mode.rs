use crate::audio::sound_effect_non_dilated;
use crate::gameplay::boomerang::{
    get_raycast_target, BoomerangHittable, BoomerangTargetKind, CurrentBoomerangThrowOrigin,
    ThrowBoomerangEvent,
};
use crate::gameplay::input::AimModeAction;
use crate::gameplay::mouse_position::MousePosition;
use crate::gameplay::player::Player;
use crate::physics_layers::GameLayer;
use avian3d::prelude::{
    Collider, Physics, PhysicsTime, ShapeCastConfig, SpatialQuery, SpatialQueryFilter,
};
use bevy::asset::{Asset, AssetServer, Handle};
use bevy::audio::AudioSource;
use bevy::color::{palettes, Color};
use bevy::ecs::entity::EntityHashSet;
use bevy::math::{Dir3, Isometry3d, Quat};
use bevy::prelude::{
    Commands, Component, Entity, Event, EventWriter, FromWorld, Gizmos, NextState, Query, Reflect,
    Res, ResMut, Resource, Single, State, States, Transform, Trigger, With, World,
};
use bevy_enhanced_input::events::{Completed, Fired};
use rand::{thread_rng, Rng};
use tracing::{debug, info, warn};

// ===================
// AIM MODE
// ==================
use crate::gameplay::enemy::Enemy;
use crate::theme::film_grain::FilmGrainSettingsTween;
use bevy::prelude::*;

/// The "minimum possible" speed time can go. We never fully pause the game during slo-mo.
pub const SLOW_MO_SCALING_FACTOR: f32 = 0.1;

pub fn plugin(app: &mut App) {
    app.init_resource::<AimModeAssets>();
    app.add_systems(
        Update,
        (draw_crosshair, draw_target_circles, draw_target_lines)
            .run_if(in_state(AimModeState::Aiming)),
    );
    app.add_systems(Update, record_target_near_mouse);
    app.add_systems(
        OnEnter(AimModeState::Aiming),
        (
            initialize_target_list,
            FilmGrainSettingsTween::tween_tunnel_vision_focus,
        ),
    );
    app.add_systems(OnExit(AimModeState::Aiming), cleanup_target_list);
    app.add_systems(
        OnExit(AimModeState::Aiming),
        (
            reset_current_boomerang_throw_origin_to_player,
            FilmGrainSettingsTween::tween_to_default_camera_settings,
        ),
    );

    app.init_state::<AimModeState>();
    app.add_observer(enter_aim_mode).add_observer(exit_aim_mode);

    // slowdown time while in aim mode
    app.add_systems(
        OnEnter(AimModeState::Aiming),
        |mut t: ResMut<Time<Physics>>| t.set_relative_speed(SLOW_MO_SCALING_FACTOR),
    );
    app.add_systems(
        OnExit(AimModeState::Aiming),
        |mut t: ResMut<Time<Physics>>| t.set_relative_speed(1.0),
    );

    app.add_observer(play_enemy_targeted_sound_effect);
    app.register_type::<AimModeTargets>();
}

// =====================
// STATE MACHINE
// =====================
#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Default)]
#[states(scoped_entities)]
pub enum AimModeState {
    #[default]
    Normal,
    Aiming,
}

pub fn enter_aim_mode(
    _trigger: Trigger<Fired<AimModeAction>>,
    state: Res<State<AimModeState>>,
    mut next_state: ResMut<NextState<AimModeState>>,
) {
    // don't enter aim mode if we're already in it
    if state.get() == &AimModeState::Aiming {
        return;
    }

    info!("Entering aim mode");
    next_state.set(AimModeState::Aiming);
}

pub fn exit_aim_mode(
    _trigger: Trigger<Completed<AimModeAction>>,
    state: Res<State<AimModeState>>,
    mut next_state: ResMut<NextState<AimModeState>>,
) {
    // we can only exit aim mode if we're in it
    if state.get() != &AimModeState::Aiming {
        return;
    }

    info!("Exiting aim mode");
    next_state.set(AimModeState::Normal);
}

// =====================
// AUDIO
// =====================
#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct AimModeAssets {
    #[dependency]
    entering_aim_mode: Handle<AudioSource>,
    #[dependency]
    targeting: Vec<Handle<AudioSource>>,
}

impl FromWorld for AimModeAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let targeting = vec![
            assets.load("audio/sound_effects/spurs/spur1.ogg"),
            assets.load("audio/sound_effects/spurs/spur2.ogg"),
            assets.load("audio/sound_effects/spurs/spur3.ogg"),
            assets.load("audio/sound_effects/spurs/spur4.ogg"),
            assets.load("audio/sound_effects/spurs/spur5.ogg"),
        ];
        Self {
            entering_aim_mode: assets
                .load("audio/sound_effects/571273__princeofworms__hawkeagle-cry-distant.ogg"),
            targeting,
        }
    }
}

#[derive(Event)]
pub struct PlayEnemyTargetedSound;

pub fn play_enemy_targeted_sound_effect(
    _trigger: Trigger<PlayEnemyTargetedSound>,
    mut commands: Commands,
    assets: Option<Res<AimModeAssets>>,
) {
    let Some(assets) = assets else {
        return;
    };

    let random_index = thread_rng().gen_range(0..assets.targeting.len());

    commands.spawn((
        Name::from("EnemyTargetSoundEffect"),
        sound_effect_non_dilated(assets.targeting[random_index].clone(), -12.),
    ));
}

// ===================
// TARGETING
// ===================
const AUTOTARGETING_RADIUS: f32 = 2.0;

#[derive(Component, Default, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AimModeTargets {
    targets: Vec<Entity>,
    // todo when aim mode exits, despawn this entity and fire a single boomerang with the list of targets we painted
}

pub fn initialize_target_list(mut commands: Commands) {
    commands.spawn((Name::from("AimModeTargets"), AimModeTargets::default()));
}

pub fn cleanup_target_list(
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
    // todo not why we nee this or how to handle multiple such entities. just assuming throws always originate from the player for now.
    let player = player_single.into_inner();
    if !v.is_empty() {
        event_writer.write(ThrowBoomerangEvent {
            thrower_entity: player,
            target: v,
        });
    }
    commands.entity(e).despawn();
}

pub fn draw_crosshair(mut gizmos: Gizmos, mouse_position: Res<MousePosition>) {
    let Some(mouse_position) = mouse_position.boomerang_throwing_plane else {
        debug!("No mouse position found");
        return;
    };

    // Create a rotation that rotates 90 degrees (PI/2 radians) around the X-axis
    let rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let isometry = Isometry3d::new(mouse_position, rotation);

    gizmos.circle(isometry, 2.0, Color::srgb(0.9, 0.1, 0.1));
}

pub fn draw_target_circles(
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
}

pub fn draw_target_lines(
    mut gizmos: Gizmos,
    hittables: Query<&Transform, With<BoomerangHittable>>,
    query: Single<&AimModeTargets>,
    player_single: Single<(Entity, &Transform), With<Player>>,
    spatial_query: SpatialQuery,
) -> Result {
    let targets = query.into_inner();
    let x = &targets.targets;

    let (mut last_entity_found, mut last_transform_found) = player_single.into_inner();

    for e in x.iter() {
        if let Ok(t) = hittables.get(*e) {
            let (mut target_entity, target_location) = match get_raycast_target(
                &spatial_query,
                t.translation,
                last_entity_found,
                last_transform_found.translation,
            ) {
                Ok(value) => value,
                Err(_value) => continue,
            };

            if let Some(te) = target_entity {
                if hittables.get(te).is_err() {
                    // If the entity hit isn't one of the targetable ones, we hit a wall.
                    target_entity = None;
                }
            }

            let color = match target_entity {
                Some(_entity) => Color::srgb(0., 1., 0.),
                None => Color::srgb(1., 0.1, 0.1),
            };

            // todo use retained mode gizmos to be more efficient (or an instanced mesh of a cool looking crosshair)
            gizmos.line(last_transform_found.translation, target_location, color);

            last_transform_found = t;
            last_entity_found = *e;
        }
    }

    Ok(())
}

const MAX_TARGETS_SELECTABLE: usize = 3;

pub fn record_target_near_mouse(
    mouse_position: Res<MousePosition>,
    spatial_query: SpatialQuery,
    mut current_target_list: Single<&mut AimModeTargets>,
    current_throw_origin: Single<(Entity, &Transform), With<CurrentBoomerangThrowOrigin>>,
    enemies_query: Query<Entity, With<Enemy>>,
    mut commands: Commands,
    mut gizmos: Gizmos,
) -> Result {
    // target list is full, don't add any more targets
    if current_target_list.targets.len() >= MAX_TARGETS_SELECTABLE {
        return Ok(());
    }

    let Some(mouse_position) = mouse_position.boomerang_throwing_plane else {
        warn!("No mouse position found");
        return Ok(());
    };
    let (origin_entity, origin_transform) = current_throw_origin.into_inner();

    let Ok(direction_from_thrower_to_cursor) =
        Dir3::new((mouse_position - origin_transform.translation).normalize_or_zero())
    else {
        return Ok(());
    };

    // Cast a sphere from the thrower to the cursor, returning the first enemy hit (this is what we're targeting).
    // The reason it's a sphere is to allow for some "auto-aim" functionality - you don't need to mouse over the target exactly.
    let Some(target_near_cursor) = spatial_query.cast_shape_predicate(
        &Collider::sphere(AUTOTARGETING_RADIUS), // Shape
        origin_transform.translation,            // Shape position
        Quat::default(),                         // Shape rotation
        direction_from_thrower_to_cursor,
        &ShapeCastConfig::from_max_distance(
            origin_transform.translation.distance(mouse_position) + AUTOTARGETING_RADIUS / 2.,
        ),
        &SpatialQueryFilter::from_mask(GameLayer::Enemy)
            .with_excluded_entities(vec![origin_entity]),
        &|e| enemies_query.contains(e),
    ) else {
        // info!("record_target_near_mouse:: no target near cursor at {:?}", mouse_position);
        return Ok(());
    };

    {
        let _dist = target_near_cursor
            .point1
            .distance(origin_transform.translation);
        // info!("record_target_near_mouse:: target near cursor {:?} away from origin of throw", dist);
    }

    // Check for intervening walls with a ray cast. This time, we don't filter to
    // Enemies only - if we hit a wall before hitting our target, we don't add
    // it to the list of targeted entities.
    {
        let Ok(ray_direction) = Dir3::new(
            (target_near_cursor.point1 - origin_transform.translation).normalize_or_zero(),
        ) else {
            // info!("record_target_near_mouse:: couldn't raycast to painted target");
            return Ok(());
        };
        let line_of_sight_ray = spatial_query.cast_ray_predicate(
            origin_transform.translation,
            ray_direction,
            900.,
            true,
            &SpatialQueryFilter {
                excluded_entities: EntityHashSet::from([origin_entity]),
                ..Default::default()
            },
            &|e| origin_entity != e,
        );
        // info!("record_target_near_mouse:: cast ray from {:?} to {:?}. Direction {:?}", origin_transform.translation, target_near_cursor.point1, ray_direction);
        gizmos.line(
            origin_transform.translation,
            target_near_cursor.point1,
            palettes::css::BLUE_VIOLET,
        );
        let Some(ray_hit) = line_of_sight_ray else {
            // info!("record_target_near_mouse:: no ray hits");
            return Ok(());
        };
        if ray_hit.entity != target_near_cursor.entity {
            // info!(
            //     "record_target_near_mouse:: ray hit a different target {:?} than the mouse cursor: {:?}",
            //     ray_hit.entity, target_near_cursor.entity
            // );
            return Ok(());
        }
    }

    // Finally, check if the targeted entity has already been targeted
    // If so, then we don't add it again.
    if current_target_list.targets.contains(&target_near_cursor.entity) {
        return Ok(());
    } else {
        swap_boomerang_throw_origin(
            origin_entity,
            target_near_cursor.entity,
            commands.reborrow(),
        );
        current_target_list.targets.push(target_near_cursor.entity);
        commands.trigger(PlayEnemyTargetedSound); // play a sound when an enemy is targeted
    }

    Ok(())
}

fn reset_current_boomerang_throw_origin_to_player(
    player: Single<Entity, With<Player>>,
    current_throw_origin: Single<Entity, With<CurrentBoomerangThrowOrigin>>,
    commands: Commands,
) {
    swap_boomerang_throw_origin(*current_throw_origin, *player, commands);
}

/// Moves the boomerang throw origin component from one entity to another
fn swap_boomerang_throw_origin(from: Entity, to: Entity, mut commands: Commands) {
    commands
        .entity(from)
        .remove::<CurrentBoomerangThrowOrigin>();
    commands.entity(to).insert(CurrentBoomerangThrowOrigin);
}
