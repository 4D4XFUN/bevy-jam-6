use bevy::prelude::App;

pub mod framepace;

pub fn plugin(app: &mut App) {
    app.add_plugins(framepace::plugin);
}