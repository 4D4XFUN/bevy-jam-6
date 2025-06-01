use bevy::app::App;
use crate::gameplay::boomerang::BoomerangThrowingPlugin;
use crate::gameplay::mouse_position::MousePositionPlugin;

mod boomerang;
mod mouse_position;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((BoomerangThrowingPlugin, MousePositionPlugin));
}