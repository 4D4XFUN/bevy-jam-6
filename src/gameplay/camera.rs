use crate::gameplay::Gameplay;
use crate::gameplay::boomerang::BounceBoomerangEvent;
use bevy::app::{App, Startup, Update};
use bevy::color::Color;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::ReflectComponent;
use bevy::prelude::{
    Camera, Camera3d, Commands, Component, Entity, EventReader, IsDefaultUiCamera, Msaa, Name,
    PerspectiveProjection, Projection, Query, Real, Reflect, Res, Single, Time, Timer, TimerMode,
    Transform, Window, With, Without, default,
};
use bevy::render::camera::Exposure;
use bevy::state::condition::in_state;
use rand::{Rng, thread_rng};

pub fn plugin(app: &mut App) {
    // systems
    app.add_systems(Startup, spawn_camera);
    app.add_systems(
        Update,
        (
            camera_follow,
            start_shake_on_boomerang_bounce,
            update_screen_shake,
            tick_shake_timers,
        )
            .run_if(in_state(Gameplay::Normal)),
    );

    // reflection
    app.register_type::<CameraProperties>();
}

#[derive(Component)]
pub struct SceneCamera;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct CameraFollowTarget;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
pub struct CameraProperties {
    camera_follow_snappiness: f32,
}

const INITIAL_Z_OFFSET: f32 = 25.0;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Msaa::Sample4,
        IsDefaultUiCamera,
        CameraProperties {
            camera_follow_snappiness: 7.0,
        },
        Transform::from_xyz(0., 40., INITIAL_Z_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            hdr: true,
            clear_color: Color::srgb_u8(15, 9, 20).into(),
            ..Default::default()
        },
        Projection::from(PerspectiveProjection {
            fov: 35.0_f32.to_radians(),
            ..default()
        }),
        // RenderLayers::from(  // this is from froxtrot but isn't working right
        //     RenderLayer::DEFAULT | RenderLayer::PARTICLES | RenderLayer::GIZMO3,
        // ),
        Exposure::INDOOR,
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
    ));
}

fn camera_follow(
    camera: Single<(&mut Transform, &CameraProperties), With<Camera>>,
    target: Single<&Transform, (With<CameraFollowTarget>, Without<Camera>)>,
    time: Res<Time<Real>>,
) -> bevy::prelude::Result {
    let target_transform = target.into_inner();
    let (mut camera_transform, properties) = camera.into_inner();

    //calculate bounds
    let level_width = 50.0f32;
    let level_height = 50.0f32;
    let min_x = -level_width / 2.0;
    let max_x = level_width / 2.0;
    let min_z = -level_height / 2.0 + INITIAL_Z_OFFSET;
    let max_z = level_height / 2.0 + INITIAL_Z_OFFSET;

    let bounded_target_position = Vec3::new(
        target_transform.translation.x.clamp(min_x, max_x),
        camera_transform.translation.y,
        (target_transform.translation.z + INITIAL_Z_OFFSET).clamp(min_z, max_z),
    );

    //smoothly interpolate camera position to target position
    let translation = camera_transform.translation.lerp(
        bounded_target_position,
        time.delta_secs() * properties.camera_follow_snappiness,
    );

    //and hard clam that camera's position if it is out of bounds
    camera_transform.translation = Vec3::new(
        translation.x.clamp(min_x, max_x),
        camera_transform.translation.y,
        translation.z.clamp(min_z, max_z),
    );

    Ok(())
}

// ===============
// SCREEN SHAKE
// ===============

#[derive(Component, Debug, Reflect, Default)]
struct ScreenShake {
    intensity: f32, // 0.0 - 1.0
    timer: Timer,
}

impl ScreenShake {
    pub fn default() -> Self {
        Self {
            intensity: 0.01,
            timer: Timer::from_seconds(0.1, TimerMode::Once),
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
    let mut rng = thread_rng();

    // Get viewport dimensions for scaling
    let viewport_size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1024.0, 768.0));
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
    time: Res<Time<Real>>,
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
