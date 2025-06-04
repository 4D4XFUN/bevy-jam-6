use bevy::prelude::*;

mod enemy_movement;
mod enemy_spawning;

pub fn plugin(app: &mut App) {
    app.add_plugins(enemy_spawning::plugin);
}

#[derive(Component)]
pub struct Enemy;
