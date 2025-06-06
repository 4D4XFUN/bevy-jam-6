use crate::audio::sound_effect;
use crate::gameplay::boomerang::{get_raycast_target, BoomerangHittable, BoomerangTargetKind, ThrowBoomerangEvent};
use crate::gameplay::input::AimModeAction;
use crate::gameplay::mouse_position::MousePosition;
use crate::gameplay::player::Player;
use crate::physics_layers::GameLayer;
use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::asset::{Asset, AssetServer, Handle};
use bevy::audio::AudioSource;
use bevy::color::Color;
use bevy::math::{Dir3, Isometry3d, Quat};
use bevy::prelude::{
    Commands, Component, Entity, Event, EventWriter, FromWorld, Gizmos, NextState, Query, Reflect,
    Res, ResMut, Resource, Single, State, States, Transform, Trigger, With, World,
};
use bevy_enhanced_input::events::{Completed, Fired};
use rand::Rng;
use tracing::{debug, info, warn};

// ===================
// AIM MODE
// ==================
use crate::gameplay::time_dilation::DilatedTime;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (draw_crosshair, draw_target_circles, draw_target_lines).run_if(in_state(AimModeState::Aiming)),
    );
    app.add_systems(Update, record_target_near_mouse);
    app.add_systems(OnEnter(AimModeState::Aiming), initialize_target_list);
    app.add_systems(OnExit(AimModeState::Aiming), cleanup_target_list);

    app.init_state::<AimModeState>();
    app.add_observer(enter_aim_mode).add_observer(exit_aim_mode);

    // slowdown time while in aim mode
    app.add_systems(
        OnEnter(AimModeState::Aiming),
        |mut t: ResMut<DilatedTime>| t.scaling_factor = DilatedTime::SLOW_MO_SCALING_FACTOR,
    );
    app.add_systems(
        OnExit(AimModeState::Aiming),
        |mut t: ResMut<DilatedTime>| t.scaling_factor = 1.0,
    );

    app.add_observer(play_enemy_targeted_sound_effect);
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
    targeting1: Handle<AudioSource>,
    #[dependency]
    targeting2: Handle<AudioSource>,
    #[dependency]
    targeting3: Handle<AudioSource>,
    #[dependency]
    targeting4: Handle<AudioSource>,
    #[dependency]
    targeting5: Handle<AudioSource>,
}

impl FromWorld for AimModeAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            entering_aim_mode: assets
                .load("audio/sound_effects/571273__princeofworms__hawkeagle-cry-distant.ogg"),

            targeting1: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting2: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting3: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting4: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting5: assets.load("audio/sound_effects/spurs/spur1.ogg"),
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

    let random_index = rand::thread_rng().gen_range(1..=5);

    let sound_asset = match random_index {
        1 => assets.targeting1.clone(),
        2 => assets.targeting2.clone(),
        3 => assets.targeting3.clone(),
        4 => assets.targeting4.clone(),
        5 => assets.targeting5.clone(),
        _ => unreachable!(),
    };

    commands.spawn(sound_effect(sound_asset));
}

// ===================
// TARGETING
// ===================
const AUTOTARGETING_RADIUS: f32 = 2.0;

#[derive(Component, Default, Debug, Clone)]
pub struct AimModeTargets {
    targets: Vec<Entity>,
    // todo when aim mode exits, despawn this entity and fire a single boomerang with the list of targets we painted
}

pub fn initialize_target_list(mut commands: Commands) {
    commands.spawn(AimModeTargets::default());
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
    let player = player_single.into_inner(); // todo not why we nee this or how to handle multiple such entities. just assuming throws always originate from the player for now.

    event_writer.write(ThrowBoomerangEvent {
        thrower_entity: player,
        target: v,
    });
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
            let (mut target_entity, target_location) = match get_raycast_target(&spatial_query, t.translation, last_entity_found, last_transform_found.translation) {
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
    mut commands: Commands,
) -> Result {
    // target list is full, don't add any more targets
    if current_target_list.targets.len() >= MAX_TARGETS_SELECTABLE {
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
            commands.trigger(PlayEnemyTargetedSound); // play a sound when an enemy is targeted
            // info!(
            //     "Adding target to list {:?}. List after addition: {:?}",
            //     hit.entity, &current_target_list.targets
            // );
        }
    }

    Ok(())
}
