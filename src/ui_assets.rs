use bevy::asset::{Asset, AssetServer, Assets, Handle};
use bevy::image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy::math::UVec2;
use bevy::prelude::ReflectResource;
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
            header: asset_server.load("fonts/RioGrande.ttf"),
            content: asset_server.load("fonts/Kirsty Rg.otf"),
        }
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct PanelAssets {
    #[dependency]
    image_handle: Handle<Image>,
    layout_handle: Handle<TextureAtlasLayout>,
    slicer: TextureSlicer,
}

impl PanelAssets {
    pub fn to_image_node(&self) -> ImageNode {
        ImageNode::from_atlas_image(
            self.image_handle.clone(),
            TextureAtlas {
                layout: self.layout_handle.clone(),
                index: 0,
            },
        )
        .with_mode(NodeImageMode::Sliced(self.slicer.clone()))
    }
}

impl FromWorld for PanelAssets {
    fn from_world(world: &mut World) -> Self {
        let layout_handle = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            let layout = TextureAtlasLayout::from_grid(UVec2::splat(128), 1, 1, None, None);
            layouts.add(layout)
        };
        let slicer = TextureSlicer {
            border: BorderRect::all(32.0),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        };
        let asset_server = world.resource::<AssetServer>();
        PanelAssets {
            image_handle: asset_server.load("images/1bit-panel.png"),
            layout_handle,
            slicer,
        }
    }
}
