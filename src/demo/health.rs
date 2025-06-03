//! This module is in charge of managing health.
//! Give an enemy, player or obj health by attaching the [`Health`] component to it, e.g. `Health(3)`, to give it 3 health points.
//! Damage an enemy, player or obj by triggering the [`HealthEvent`] on an entity, e.g. `HealthEvent::Damage(1)` to reduce health by one.
//! Listen to the [`DeathEvent`] on the entity to handle special cases, like Game Over screen, ragdolling or exploding.
use bevy::prelude::*;

#[derive(Event)]
pub enum HealthEvent {
    Damage(i32),
}

#[derive(Event)]
pub struct DeathEvent;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Health(pub i32);

#[derive(Component)]
pub struct HealthUi(Entity);

pub fn plugin(app: &mut App) {
    app.register_type::<Health>()
        .add_event::<HealthEvent>()
        .add_event::<DeathEvent>()
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
        HealthEvent::Damage(dmg) => health.0 -= dmg,
    }
    println!("Health: {:?}", health.0);
    if health.0 <= 0 {
        commands
            .entity(trigger.target())
            .remove::<Health>()
            .trigger(DeathEvent);
    }
}
