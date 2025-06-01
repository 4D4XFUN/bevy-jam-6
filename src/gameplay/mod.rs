use crate::asset_tracking::LoadResource;
use crate::assets::MeshAssets;
use crate::gameplay::boomerang::BoomerangThrowingPlugin;
use crate::gameplay::mouse_position::MousePositionPlugin;
use bevy::app::App;

mod boomerang;
mod mouse_position;

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<MeshAssets>();
    app.add_plugins((BoomerangThrowingPlugin, MousePositionPlugin));
}
