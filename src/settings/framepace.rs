use bevy::prelude::*;
use bevy_framepace::{FramepaceSettings, Limiter};

pub fn plugin(app: &mut App) {
    app.add_plugins(bevy_framepace::FramepacePlugin);
    app.add_systems(Startup, limit_fps);
}

fn limit_fps(mut fps_settings: ResMut<FramepaceSettings>) {
    let max_fps = 60.0;
    fps_settings.limiter = Limiter::from_framerate(max_fps);
    info!("FPS limit set to {}", max_fps);
}
