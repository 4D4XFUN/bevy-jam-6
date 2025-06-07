//! This module is in charge of managing health.
//! Give an enemy, player or obj health by attaching the [`Health`] component to it, e.g. `Health(3)`, to give it 3 health points.
//! Damage an enemy, player or obj by triggering the [`HealthEvent`] on an entity, e.g. `HealthEvent::Damage(1)` to reduce health by one.
//! Listen to the [`DeathEvent`] on the entity to handle special cases, like Game Over screen, ragdolling or exploding.

use avian3d::prelude::{
    AngularVelocity, Collider, CollisionLayers, CollisionStarted, LinearVelocity, PhysicsLayer,
    RigidBody,
};
use bevy::prelude::*;
use rand::{Rng, thread_rng};

use crate::{asset_tracking::LoadResource, physics_layers::GameLayer, screens::Screen};

#[derive(Event)]
pub enum HealthEvent {
    Damage(u32),
}

#[derive(Event)]
pub struct DeathEvent;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Health(pub i32);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CanDamage(pub u32);

#[derive(Component)]
pub struct HealthUi(Entity);

pub fn plugin(app: &mut App) {
    app.register_type::<Health>()
        .add_event::<HealthEvent>()
        .add_event::<DeathEvent>()
        .load_resource::<HealthAsset>()
        .add_systems(Update, on_damage_event)
        .add_systems(PostUpdate, move_ui)
        .add_observer(add_health_ui)
        .add_observer(remove_health_ui)
        .add_observer(on_health_event);
}

fn add_health_ui(
    trigger: Trigger<OnAdd, Health>,
    health_asset: Res<HealthAsset>,
    health_carriers: Query<&Transform, With<Health>>,
    mut commands: Commands,
) {
    let Ok(transform) = health_carriers.get(trigger.target()) else {
        return;
    };
    commands.spawn((
        Name::from("Hat"),
        StateScoped(Screen::Gameplay),
        SceneRoot(health_asset.0.clone()),
        HealthUi(trigger.target()),
        Transform::from_translation(transform.translation + Vec3::Y),
    ));
}

fn remove_health_ui(
    trigger: Trigger<OnRemove, Health>,
    health_uis: Query<(Entity, &HealthUi)>,
    mut commands: Commands,
) {
    let mut rand = thread_rng();
    let random_velocity: Vec3 = rand.r#gen();
    if let Some((entity, _)) = health_uis.iter().find(|(_, ui)| ui.0 == trigger.target()) {
        commands
            .entity(entity)
            .insert((
                LinearVelocity(Vec3::Y * 5.),
                AngularVelocity(random_velocity.normalize() * 5.0),
                RigidBody::Dynamic,
                Collider::cuboid(1.6, 0.4, 1.6),
                CollisionLayers::new(GameLayer::DeadEnemy, GameLayer::all_bits()),
            ))
            .remove::<HealthUi>();
    }
}

fn move_ui(
    healths: Query<&Transform, Without<HealthUi>>,
    mut uis: Query<(&mut Transform, &HealthUi)>,
) {
    for (mut transform, health_ui) in &mut uis {
        let Ok(health_transform) = healths.get(health_ui.0) else {
            continue;
        };
        transform.translation = health_transform.translation + Vec3::Y;
        transform.rotation = health_transform.rotation;
    }
}

fn on_health_event(
    trigger: Trigger<HealthEvent>,
    mut health: Query<&mut Health>,
    mut commands: Commands,
) {
    let Ok(mut health) = health.get_mut(trigger.target()) else {
        return;
    };
    match trigger.event() {
        HealthEvent::Damage(dmg) => health.0 -= *dmg as i32,
    }
    if health.0 <= 0 {
        commands
            .entity(trigger.target())
            .remove::<Health>()
            .trigger(DeathEvent);
    }
}

fn on_damage_event(
    mut collision_event: EventReader<CollisionStarted>,
    health_query: Query<Entity, With<Health>>,
    damager_query: Query<(Entity, &CanDamage)>,
    mut commands: Commands,
) {
    for CollisionStarted(entity1, entity2) in collision_event.read() {
        for health_entity in health_query.iter() {
            for (damager_entity, damager) in damager_query.iter() {
                if (*entity1 == health_entity || *entity2 == health_entity)
                    && (*entity1 == damager_entity || *entity2 == damager_entity)
                {
                    commands
                        .entity(health_entity)
                        .trigger(HealthEvent::Damage(damager.0));
                }
            }
        }
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct HealthAsset(Handle<Scene>);

impl FromWorld for HealthAsset {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        HealthAsset(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/hat_stetson.glb")),
        )
    }
}
