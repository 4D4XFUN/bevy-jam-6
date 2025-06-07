use crate::gameplay::Gameplay;
use crate::gameplay::enemy::Enemy;
use crate::gameplay::health_and_damage::{CanDamage, Health, HealthEvent};
use crate::gameplay::input::FireBoomerangAction;
use crate::gameplay::mouse_position::MousePosition;
use crate::physics_layers::GameLayer;
use avian3d::prelude::{
    AngularVelocity, Collider, CollisionEventsEnabled, CollisionLayers, LinearVelocity, Physics,
    PhysicsTime, RigidBody,
};
use avian3d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::color;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::Fired;
use rand::{Rng, thread_rng};

pub const BOOMERANG_FLYING_HEIGHT: f32 = 0.5;

/// Component used to describe boomerang entities.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
struct Boomerang {
    /// The path this boomerang is following.
    path: Vec<BoomerangTargetKind>,
    path_index: usize,
    progress_on_current_segment: f32, // value from 0.0 to 1.0
}
impl Boomerang {
    fn new(path: Vec<BoomerangTargetKind>) -> Self {
        Self {
            path,
            path_index: 0,
            progress_on_current_segment: 0.0,
        }
    }

    fn _is_last_segment(&self) -> bool {
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

/// Entities with this component will allow the user to redirect the boomerang bounce when they are hit by becoming a [CurrentBoomerangThrowOrigin]
#[derive(Component, Default)]
#[require(BoomerangHittable)]
pub struct PotentialBoomerangOrigin;

/// Component which should be added to the entity the boomerang is currently "attached" to.
/// Used to mark the origin for the next bounce direction. There should always be one (and exactly one) entity with this component during a running game.
#[derive(Component)]
#[require(PotentialBoomerangOrigin)]
pub struct CurrentBoomerangThrowOrigin;

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
struct BoomerangAssets {
    mesh: Handle<Scene>,
    toss_sfx: Vec<Handle<AudioSource>>,
    loop_sfx: Handle<AudioSource>,
}

impl FromWorld for BoomerangAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let toss_sfx = vec![
            asset_server.load("audio/sound_effects/boomerang_sfx/boomerang_toss1.ogg"),
            asset_server.load("audio/sound_effects/boomerang_sfx/boomerang_toss2.ogg"),
            asset_server.load("audio/sound_effects/boomerang_sfx/boomerang_toss3.ogg"),
        ];
        BoomerangAssets {
            mesh: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/boomerang.glb")),
            toss_sfx,
            loop_sfx: asset_server
                .load("audio/sound_effects/boomerang_sfx/boomerang_loop_single_short.ogg"),
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
            update_sfx_speed,
        )
            .run_if(in_state(Gameplay::Normal)),
    );

    app.add_observer(on_fire_action_throw_boomerang)
        .add_observer(handle_boomerang_sfx);
}

/// Moves boomerangs along their paths.
/// Fires a [BounceBoomerangEvent] in case that the next path destination was reached.
fn move_flying_boomerangs(
    mut flying_boomerangs: Query<(Entity, &mut Boomerang, &mut Transform), With<Flying>>,
    all_other_transforms: Query<&Transform, Without<Boomerang>>,
    boomerang_settings: Res<BoomerangSettings>,
    time: Res<Time<Physics>>,
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
                .remove::<BoomerangSfx>()
                .insert(Falling);
            info!("falling");
        }
    }

    Ok(())
}

/// Rotates our boomerangs at constant speed.
fn set_boomerang_rotation_speed_based_on_velocity(
    mut boomerangs: Query<(&mut AngularVelocity, &Boomerang), With<Flying>>,
    settings: Res<BoomerangSettings>,
) {
    for (mut rotation, boomerang) in boomerangs.iter_mut() {
        let rotation_speed = settings.tween_rotation_speed(boomerang.progress_on_current_segment);
        rotation.0 = Vec3::new(0.0, rotation_speed, 0.0);
    }
}

fn update_boomerang_preview_position(
    boomerang_origins: Single<(Entity, &GlobalTransform), With<CurrentBoomerangThrowOrigin>>,
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

    let (mut target_entity, target_location) = match get_raycast_target(
        &spatial_query,
        mouse_position,
        origin_entity,
        origin_transform.translation(),
    ) {
        Ok(value) => value,
        Err(_value) => return Ok(()),
    };

    if let Some(te) = target_entity {
        if potential_origins.get(te).is_err() {
            // If the entity hit isn't one of the targetable ones, we hit a wall.
            target_entity = None;
        }
    }

    if let Ok((mut preview, mut transform)) = previews.single_mut() {
        preview.target_entity = target_entity;
        transform.translation = target_location;
    } else {
        // TODO: Preview needs to be despawned after throw
        commands.spawn((
            Name::from("WeaponTarget"),
            WeaponTarget { target_entity },
            Transform::from_translation(target_location),
        ));
    }
    Ok(())
}

pub fn get_raycast_target(
    spatial_query: &SpatialQuery,
    target_position: Vec3,
    origin_entity: Entity,
    origin_transform: Vec3,
) -> Result<(Option<Entity>, Vec3), Result> {
    let origin = origin_transform.with_y(BOOMERANG_FLYING_HEIGHT);

    let Ok(direction) = Dir3::new(target_position - origin) else {
        // We are probably just pointing right at the ThrowOrigin
        return Err(Ok(()));
    };

    let max_distance = 50.0;
    let solid = true;
    let filter = SpatialQueryFilter {
        excluded_entities: EntityHashSet::from([origin_entity]),
        ..Default::default()
    };
    let (distance_to_target, target_entity) = if let Some(first_hit) =
        spatial_query.cast_ray(origin, direction, max_distance, solid, &filter)
    {
        (first_hit.distance, Some(first_hit.entity))
    } else {
        (max_distance, None)
    };

    let target_location = origin + direction * distance_to_target;
    Ok((target_entity, target_location))
}

fn on_fire_action_throw_boomerang(
    _trigger: Trigger<Fired<FireBoomerangAction>>,
    boomerang_holders: Query<Entity, With<CurrentBoomerangThrowOrigin>>,
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

#[derive(Component)]
struct BoomerangSfx(f32);

fn on_throw_boomerang_spawn_boomerang(
    mut event_reader: EventReader<ThrowBoomerangEvent>,
    mut commands: Commands,
    all_transforms: Query<&Transform>,
    boomerang_assets: Res<BoomerangAssets>,
) -> Result {
    let mut rng = thread_rng();
    for event in event_reader.read() {
        // add the thrower as both the first and last node on the path
        let thrower = BoomerangTargetKind::Entity(event.thrower_entity);
        let mut path = vec![thrower];
        path.append(&mut event.target.clone());
        path.push(thrower);

        let random_index = rng.gen_range(0..boomerang_assets.toss_sfx.len());
        let random_sfx = &boomerang_assets.toss_sfx[random_index];
        // spawn the 'rang
        commands
            .spawn((
                Name::new("Boomerang"),
                Boomerang::new(path),
                Transform::from_translation(
                    all_transforms
                        .get(event.thrower_entity)?
                        .translation
                        .with_y(BOOMERANG_FLYING_HEIGHT),
                ),
                StateScoped(Gameplay::Normal),
                Flying,
                SceneRoot(boomerang_assets.mesh.clone()),
                Collider::sphere(0.5),
                CollisionLayers::new(GameLayer::Boomerang, GameLayer::Enemy),
                RigidBody::Kinematic,
                CanDamage(1),
                CollisionEventsEnabled,
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
            ))
            .insert((
                AudioPlayer::new(random_sfx.clone()),
                PlaybackSettings::REMOVE,
                BoomerangSfx(1.0),
            ));
    }

    Ok(())
}

fn handle_boomerang_sfx(
    trigger: Trigger<OnRemove, PlaybackSettings>,
    boomerang_assets: Res<BoomerangAssets>,
    boomerang_sfx: Query<Entity, With<BoomerangSfx>>,
    mut commands: Commands,
) {
    let mut rng = thread_rng();
    if boomerang_sfx.contains(trigger.target()) {
        let pitch = rng.r#gen::<f32>() * 0.4;
        commands.entity(trigger.target()).try_insert((
            AudioPlayer::new(boomerang_assets.loop_sfx.clone()),
            PlaybackSettings::REMOVE.with_speed(0.8 + pitch),
            BoomerangSfx(0.8 + pitch),
        ));
    }
}

fn update_sfx_speed(time: Res<Time<Physics>>, query: Query<(&AudioSink, &BoomerangSfx)>) {
    for (sink, sfx) in &query {
        sink.set_speed(time.relative_speed() * sfx.0);
    }
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
    boomerang_holders: Query<&GlobalTransform, With<CurrentBoomerangThrowOrigin>>,
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
struct BoomerangSettings {
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
    fn tween_movement_speed(&self, progress: f32) -> f32 {
        self.tween_values(self.min_movement_speed, self.max_movement_speed, progress)
    }
    fn tween_rotation_speed(&self, progress: f32) -> f32 {
        self.tween_values(self.min_rotation_speed, self.max_rotation_speed, progress)
    }

    fn tween_values(&self, min: f32, max: f32, progress: f32) -> f32 {
        EasingCurve::new(min, max, self.easing_function)
            .sample(progress)
            .unwrap_or((min + max) / 2.)
    }
}
