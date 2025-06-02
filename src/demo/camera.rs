use crate::demo::aim_mode::AimModeState;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::Exposure;

pub fn plugin(app: &mut App) {
    app.register_type::<CameraProperties>();
    app.add_systems(Startup, spawn_camera);
    app.add_systems(Update, camera_follow);

    app.add_systems(OnEnter(AimModeState::Aiming), camera_enter_aim_mode);
    app.add_systems(OnExit(AimModeState::Aiming), camera_exit_aim_mode);
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
    time: Res<Time>,
) -> Result {
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

// ================
// AIM MODE
// ================
fn camera_enter_aim_mode(mut camera: Single<&mut Transform, With<Camera>>) {
    // just a really subtle zoom out when aiming
    camera.into_inner().scale.z = 0.97;
}
fn camera_exit_aim_mode(mut camera: Single<&mut Transform, With<Camera>>) {
    camera.into_inner().scale.z = 1.0;
}
