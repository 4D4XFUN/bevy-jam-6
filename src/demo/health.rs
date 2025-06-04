//! This module is in charge of managing health.
//! Give an enemy, player or obj health by attaching the [`Health`] component to it, e.g. `Health(3)`, to give it 3 health points.
//! Damage an enemy, player or obj by triggering the [`HealthEvent`] on an entity, e.g. `HealthEvent::Damage(1)` to reduce health by one.
//! Listen to the [`DeathEvent`] on the entity to handle special cases, like Game Over screen, ragdolling or exploding.

use crate::physics_layers::GameLayer;
use avian3d::prelude::{CollisionLayers, CollisionStarted};
use bevy::prelude::*;

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
        .add_systems(Update, on_damage_event)
        .add_systems(PostUpdate, update_health_ui)
        .add_observer(add_health_ui)
        .add_observer(remove_health_ui)
        .add_observer(on_health_event);
}

fn add_health_ui(trigger: Trigger<OnAdd, Health>, mut commands: Commands) {
    commands.entity(trigger.target()).with_children(|parent| {
        parent.spawn(HealthUi(trigger.target()));
    });
}

fn remove_health_ui(
    trigger: Trigger<OnRemove, Health>,
    uis: Query<(Entity, &HealthUi)>,
    mut commands: Commands,
) {
    if let Some((target, _)) = uis.iter().find(|(_, ui)| ui.0 == trigger.target()) {
        commands.entity(target).despawn();
    }
}

fn update_health_ui(healths: Query<(Entity, &Health), Changed<Health>>, uis: Query<&HealthUi>) {
    for (entity, _health) in &healths {
        let Some(_health_ui) = uis.iter().find(|ui| ui.0 == entity) else {
            continue;
        };
    }
}

fn on_health_event(
    trigger: Trigger<HealthEvent>,
    mut health: Query<&mut Health>,
    mut commands: Commands,
) {
    println!("Health Trigger Called");
    let Ok(mut health) = health.get_mut(trigger.target()) else {
        println!("Could not find health trigger target");
        return;
    };
    match trigger.event() {
        HealthEvent::Damage(dmg) => health.0 -= *dmg as i32,
    }
    println!("Health: {:?}", health.0);
    if health.0 <= 0 {
        commands
            .entity(trigger.target())
            .remove::<Health>()
            .trigger(DeathEvent)
            .insert(CollisionLayers::new(GameLayer::Terrain, GameLayer::ALL));
    }
}

fn on_damage_event(
    mut collision_event: EventReader<CollisionStarted>,
    health_query: Query<Entity, With<Health>>,
    damager_query: Query<(Entity, &CanDamage)>,
    mut commands: Commands,
) {
    for CollisionStarted(entity1, entity2) in collision_event.read() {
        println!("Collision Event Read");
        for health_entity in health_query.iter() {
            println!("Heath Found for entity {:?}", health_entity);
            for (damager_entity, damager) in damager_query.iter() {
                println!("Damager Found for entity {:?}", damager_entity);
                if (*entity1 == health_entity || *entity2 == health_entity) && (*entity1 == damager_entity || *entity2 == damager_entity) {
                    println!("FIRING EVENT FOR DAMAGE {:?}", damager.0);
                    commands.entity(health_entity).trigger(HealthEvent::Damage(damager.0));
                }
            }
        }
    }
}