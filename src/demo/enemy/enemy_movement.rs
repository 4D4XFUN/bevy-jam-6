use bevy::prelude::ReflectComponent;
use bevy::prelude::{Component, Reflect};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PlayerSeeking;