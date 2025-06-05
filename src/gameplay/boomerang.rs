use avian3d::dynamics::integrator::IntegrationSet::Velocity;
use crate::asset_tracking::LoadResource;
use crate::audio::sound_effect;
use crate::gameplay::enemy::Enemy;
use crate::gameplay::health_and_damage::{CanDamage, Health, HealthEvent};
use crate::gameplay::input::{AimModeAction, FireBoomerangAction};
use crate::gameplay::mouse_position::MousePosition;
use crate::gameplay::player::Player;
use crate::physics_layers::GameLayer;
use crate::screens::Screen;
use avian3d::prelude::{
    Collider, CollisionEventsEnabled, CollisionLayers, RigidBody, ShapeCastConfig,
};
use avian3d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::color;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use bevy_enhanced_input::events::Completed;
use bevy_enhanced_input::prelude::Fired;
use rand::Rng;
use crate::gameplay::time_dilation::{DilatedTime, RotationDilated, VelocityDilated};

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

/// Component used to mark anything that can be hit by the boomerang.
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
#[derive(Component, Default)]
pub struct WeaponTarget {
    /// The entity that's being targeted, if there is any.
    pub target_entity: Option<Entity>,
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct BoomerangAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

impl FromWorld for BoomerangAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        BoomerangAssets {
            mesh: asset_server.add(Mesh::from(Cuboid::new(1., 0.3, 0.3))),
            material: asset_server.add(StandardMaterial::from_color(Color::linear_rgb(
                0.6, 0.6, 0.0,
            ))),
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<BoomerangSettings>();
    app.register_type::<BoomerangSettings>();

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
            set_boomerang_rotation_speed_based_on_velocity,
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

    app.add_systems(
        Update,
        (draw_crosshair, draw_target_circles).run_if(in_state(AimModeState::Aiming)),
    );
    app.add_systems(Update, record_target_near_mouse);
    app.add_systems(OnEnter(AimModeState::Aiming), initialize_target_list);
    app.add_systems(OnExit(AimModeState::Aiming), cleanup_target_list);

    app.init_state::<AimModeState>();
    app.add_observer(enter_aim_mode).add_observer(exit_aim_mode);

    // sound effect!
    app.load_resource::<AimModeAssets>()
        .add_systems(OnEnter(AimModeState::Aiming), play_aim_mode_sound_effect);
    app.add_observer(play_enemy_targeted_sound_effect);
}

/// Moves boomerangs along their paths.
/// Fires a [BounceBoomerangEvent] in case that the next path destination was reached.
fn move_flying_boomerangs(
    mut flying_boomerangs: Query<(Entity, &mut Boomerang, &mut Transform), With<Flying>>,
    all_other_transforms: Query<&Transform, Without<Boomerang>>,
    boomerang_settings: Res<BoomerangSettings>,
    time: Res<DilatedTime>,
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
fn set_boomerang_rotation_speed_based_on_velocity(
    mut boomerangs: Query<(&mut RotationDilated, &Boomerang), With<Flying>>,
    settings: Res<BoomerangSettings>,
) {
    for (mut rotation, boomerang) in boomerangs.iter_mut() {
        let rotation_speed = settings.tween_rotation_speed(boomerang.progress_on_current_segment);
        rotation.0 = rotation_speed;
    }
}

fn update_boomerang_preview_position(
    boomerang_origins: Single<(Entity, &GlobalTransform), With<ActiveBoomerangThrowOrigin>>,
    potential_origins: Query<(), With<PotentialBoomerangOrigin>>,
    mut previews: Query<(&mut WeaponTarget, &mut Transform), Without<Enemy>>,
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
            WeaponTarget { target_entity },
            Transform::from_translation(target_location),
        ));
    }
    Ok(())
}

fn on_fire_action_throw_boomerang(
    _trigger: Trigger<Fired<FireBoomerangAction>>,
    boomerang_holders: Query<Entity, With<ActiveBoomerangThrowOrigin>>,
    boomerang_previews: Query<(&WeaponTarget, &GlobalTransform), Without<Enemy>>,
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
            Boomerang::new(path),
            Transform::from_translation(
                all_transforms
                    .get(event.thrower_entity)?
                    .translation
                    .with_y(BOOMERANG_FLYING_HEIGHT),
            ),
            Flying,
            Mesh3d(boomerang_assets.mesh.clone()),
            MeshMaterial3d(boomerang_assets.material.clone()),
            Collider::sphere(0.5),
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy),
            RigidBody::Kinematic,
            CanDamage(1),
            CollisionEventsEnabled,
            VelocityDilated(Vec3::ZERO),
            RotationDilated(0.0),
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
        let BoomerangTargetKind::Entity(target) = event._bounce_on else {
            continue;
        };
        if healths.contains(target) {
            commands.entity(target).trigger(HealthEvent::Damage(1));
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct BoomerangPreviewGizmos;

fn draw_preview_gizmo(
    mut gizmos: Gizmos<BoomerangPreviewGizmos>,
    boomerang_holders: Query<&GlobalTransform, With<ActiveBoomerangThrowOrigin>>,
    boomerang_target_preview: Query<&GlobalTransform, (With<WeaponTarget>, Without<Enemy>)>,
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

// ================
// SETTINGS
// ===============

#[cfg(feature = "dev")]
pub fn boomerang_dev_tools_plugin(app: &mut App) {
    use bevy_inspector_egui::quick::ResourceInspectorPlugin;
    app.add_plugins(ResourceInspectorPlugin::<BoomerangSettings>::default());
}

/// Current set of stats of our boomerang
#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct BoomerangSettings {
    pub min_movement_speed: f32,
    pub max_movement_speed: f32,
    pub min_rotation_speed: f32,
    pub max_rotation_speed: f32,
    pub falling_speed: f32,
    pub easing_function: EaseFunction, // see https://bevyengine.org/examples/animation/easing-functions/
}

impl Default for BoomerangSettings {
    fn default() -> Self {
        Self {
            min_movement_speed: 8.,
            max_movement_speed: 18.,
            min_rotation_speed: 10.,
            max_rotation_speed: 25.,
            falling_speed: 5.0,
            easing_function: EaseFunction::BackOut,
        }
    }
}

impl BoomerangSettings {
    pub fn tween_movement_speed(&self, progress: f32) -> f32 {
        self.tween_values(self.min_movement_speed, self.max_movement_speed, progress)
    }
    pub fn tween_rotation_speed(&self, progress: f32) -> f32 {
        self.tween_values(self.min_rotation_speed, self.max_rotation_speed, progress)
    }

    fn tween_values(&self, min: f32, max: f32, progress: f32) -> f32 {
        EasingCurve::new(min, max, self.easing_function)
            .sample(progress)
            .unwrap_or((min + max) / 2.)
    }
}

// ===================
// AIM MODE
// ==================

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

fn enter_aim_mode(
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

fn exit_aim_mode(
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
struct AimModeAssets {
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

fn play_aim_mode_sound_effect(mut commands: Commands, assets: Option<Res<AimModeAssets>>) {
    let Some(assets) = assets else {
        return;
    };
    commands.spawn(sound_effect(assets.entering_aim_mode.clone()));
}

#[derive(Event)]
pub struct PlayEnemyTargetedSound;

fn play_enemy_targeted_sound_effect(
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

// some absolute max that should never be reached during real gameplay (once we implement boomerang energy)
const MAX_TARGETS_SELECTABLE: usize = 30;

fn record_target_near_mouse(
    mouse_position: Res<MousePosition>,
    spatial_query: SpatialQuery,
    mut current_target_list: Single<&mut AimModeTargets>,
    mut commands: Commands,
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
            commands.trigger(PlayEnemyTargetedSound); // play a sound when an enemy is targeted
            // info!(
            //     "Adding target to list {:?}. List after addition: {:?}",
            //     hit.entity, &current_target_list.targets
            // );
        }
    }

    Ok(())
}
