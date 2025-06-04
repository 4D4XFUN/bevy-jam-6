use bevy::prelude::*;

mod enemy_spawning;
mod enemy_movement;

pub fn plugin(app: &mut App) {
    app.add_plugins(enemy_spawning::plugin);
}

#[derive(Component)]
pub struct Enemy;
