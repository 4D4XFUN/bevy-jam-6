use crate::demo::boomerang::BounceBoomerangEvent;
use bevy::prelude::*;
use rand::Rng;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, start_shake_on_boomerang_bounce);
    app.add_systems(Update, (update_screen_shake, tick_shake_timers));
}

#[derive(Component, Debug, Reflect, Default)]
struct ScreenShake {
    intensity: f32, // 0.0 - 1.0
    timer: Timer,
}

impl ScreenShake {
    pub fn default() -> Self {
        Self {
            intensity: 0.02,
            timer: Timer::from_seconds(0.25, TimerMode::Once),
        }
    }
}
fn start_shake_on_boomerang_bounce(
    mut event_reader: EventReader<BounceBoomerangEvent>,
    mut commands: Commands,
) {
    for _ in event_reader.read() {
        commands.spawn((Name::new("ScreenShake"), ScreenShake::default()));
    }
}

fn update_screen_shake(
    query: Query<&ScreenShake>,
    mut camera_query: Single<&mut Transform, With<Camera>>,
    windows: Query<&Window>,
) {
    let mut rng = rand::thread_rng();

    // Get viewport dimensions for scaling
    let viewport_size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    // Maximum shake offset as a small percentage of viewport (e.g., 1% of viewport size)
    let max_shake_percentage = 0.01;
    let max_offset = viewport_size * max_shake_percentage;

    // we only care about one shake, maybe adjust this to pick the most intense one at any given time.
    let Some(shake) = query.iter().next() else {
        return;
    };

    // Calculate shake progress (1.0 at start, 0.0 at end for decay)
    let progress = 1.0 - shake.timer.fraction();

    // Generate random offset scaled by intensity and progress
    let offset_x = rng.gen_range(-1.0..1.0) * max_offset.x * shake.intensity * progress;
    let offset_z = rng.gen_range(-1.0..1.0) * max_offset.y * shake.intensity * progress;

    let total_offset = Vec3::new(offset_x, 0.0, offset_z);

    camera_query.translation += total_offset;
}

fn tick_shake_timers(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut ScreenShake)>,
) {
    for (e, mut shake) in query.iter_mut() {
        shake.timer.tick(time.delta());
        if shake.timer.finished() {
            commands.entity(e).despawn();
        }
    }
}
