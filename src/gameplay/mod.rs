//! Demo gameplay. All of these modules are only intended for demonstration
//! purposes and should be replaced with your own game logic.
//! Feel free to change the logic found here if you feel like tinkering around
//! to get a feeling for the template.

use bevy::prelude::*;

mod aim_mode;
pub(crate) mod boomerang;
mod camera;
mod enemy;
mod health_and_damage;
mod input;
pub mod level;
mod mouse_position;
mod player;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        camera::plugin,
        level::plugin,
        input::plugin,
        player::plugin,
        mouse_position::plugin,
        boomerang::plugin,
        aim_mode::plugin,
        enemy::plugin,
        health_and_damage::plugin,
    ));
}
