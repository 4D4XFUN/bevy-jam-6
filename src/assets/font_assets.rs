use bevy::prelude::*;

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct FontAssets {
    pub header: Handle<Font>,
    pub content: Handle<Font>,
}

impl FromWorld for FontAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        FontAssets {
            header: asset_server.load("fonts/Kirsty Rg.otf"),
            content: asset_server.load("fonts/Kirsty Rg.otf"),
        }
    }
}
