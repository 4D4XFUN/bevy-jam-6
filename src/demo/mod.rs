//! Demo gameplay. All of these modules are only intended for demonstration
//! purposes and should be replaced with your own game logic.
//! Feel free to change the logic found here if you feel like tinkering around
//! to get a feeling for the template.

use bevy::prelude::*;

mod aim_mode;
mod animation;
mod boomerang;
mod camera;
mod enemy;
mod input;
pub mod level;
mod mouse_position;
mod player;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        camera::plugin,
        animation::plugin,
        level::plugin,
        player::plugin,
        input::plugin,
        mouse_position::plugin,
        boomerang::plugin,
        enemy::plugin,
        aim_mode::plugin,
    ));
}
