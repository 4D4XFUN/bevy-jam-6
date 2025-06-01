use crate::gameplay::boomerang::BOOMERANG_FLYING_HEIGHT;
use bevy::app::{App, Plugin, PreUpdate};
use bevy::math::Vec3;
use bevy::prelude::{
    Camera, GlobalTransform, InfinitePlane3d, Query, ResMut, Resource, Vec2, Window, With,
};
use bevy::window::PrimaryWindow;

/// The current position our mouse is pointing at.
#[derive(Resource, Default)]
pub struct MousePosition {
    /// The position in screen space coordinates.
    pub screen: Option<Vec2>,
    /// The position in global space on the Y=0 plane.
    pub global: Option<Vec3>,
    /// The position in global space on the boomerang-throwing plane.
    pub boomerang_throwing_plane: Option<Vec3>,
}

impl MousePosition {
    fn reset(&mut self) {
        self.screen = None;
        self.global = None;
    }
}

pub(crate) struct MousePositionPlugin;
impl Plugin for MousePositionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MousePosition>();
        app.add_systems(PreUpdate, update_mouse_position);
    }
}

/// Taken & adjusted from https://bevy-cheatbook.github.io/cookbook/cursor2world.html
fn update_mouse_position(
    mut mouse_position: ResMut<MousePosition>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    mouse_position.reset();

    // get the camera info and transform
    // assuming there is exactly one main camera entity, so Query::single() is OK
    let (camera, camera_transform) = camera_query.single().unwrap();

    // There is only one primary window, so we can similarly get it from the query:
    let window = window_query.single().unwrap();

    // check if the cursor is inside the window and get its position
    let Some(cursor_position) = window.cursor_position() else {
        // if the cursor is not inside the window, we can't do anything
        return;
    };

    mouse_position.boomerang_throwing_plane = planecast(
        camera,
        camera_transform,
        cursor_position,
        BOOMERANG_FLYING_HEIGHT,
    );
    mouse_position.global = planecast(camera, camera_transform, cursor_position, 0.0);
}

fn planecast(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_position: Vec2,
    plane_height: f32,
) -> Option<Vec3> {
    // Mathematically, we can represent the ground as an infinite flat plane.
    // To do that, we need a point (to position the plane) and a normal vector
    // (the "up" direction, perpendicular to the ground plane).

    // We can get the correct values from the ground entity's GlobalTransform
    // I'm assuming our ground plane is at Y=0 and pointing upwards.
    let plane_origin = Vec3::Y * plane_height;
    let plane = InfinitePlane3d::default();

    // Ask Bevy to give us a ray pointing from the viewport (screen) into the world
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        // if it was impossible to compute for whatever reason; we can't do anything
        return None;
    };

    // do a ray-plane intersection test, giving us the distance to the ground
    let Some(distance) = ray.intersect_plane(plane_origin, plane) else {
        // If the ray does not intersect the ground
        // (the camera is not looking towards the ground), we can't do anything
        return None;
    };

    // use the distance to compute the actual point on the ground in world-space
    let global_cursor = ray.get_point(distance);
    Some(global_cursor)
}
