//! Demo gameplay. All of these modules are only intended for demonstration
//! purposes and should be replaced with your own game logic.
//! Feel free to change the logic found here if you feel like tinkering around
//! to get a feeling for the template.

use bevy::prelude::*;

use crate::screens::Screen;

pub mod aim_mode;
mod ammo;
pub(crate) mod boomerang;
pub mod camera;
pub mod enemy;
mod footsteps;
pub mod health_and_damage;
pub mod input;
pub mod level;
pub mod mouse_position;
pub mod player;
mod score;

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Gameplay>().add_plugins((
        camera::plugin,
        level::plugin,
        input::plugin,
        player::plugin,
        mouse_position::plugin,
        boomerang::plugin,
        aim_mode::plugin,
        enemy::plugin,
        health_and_damage::plugin,
        score::plugin,
        ammo::plugin,
        footsteps::plugin,
    ));
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, SubStates)]
#[source(Screen = Screen::Gameplay)]
#[states(scoped_entities)]
pub enum Gameplay {
    #[default]
    Normal,
    GameOver,
}
