use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::Exposure;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_camera);
}

#[derive(Component)]
pub struct SceneCamera;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Msaa::Sample4,
        IsDefaultUiCamera,
        Transform::from_xyz(100., 100., 100.).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            hdr: true,
            clear_color: Color::srgb_u8(15, 9, 20).into(),
            ..Default::default()
        },
        Projection::from(PerspectiveProjection {
            fov: 5.0_f32.to_radians(),
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
