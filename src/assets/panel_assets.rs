use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

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
                index: 74,
            },
        )
        .with_mode(NodeImageMode::Sliced(self.slicer.clone()))
    }
}

impl FromWorld for PanelAssets {
    fn from_world(world: &mut World) -> Self {
        let layout_handle = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 10, 8, None, None);
            layouts.add(layout)
        };
        let slicer = TextureSlicer {
            border: BorderRect::all(16.0),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        };
        let asset_server = world.resource::<AssetServer>();
        PanelAssets {
            image_handle: asset_server.load_with_settings(
                "images/Ram Border All.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            layout_handle,
            slicer,
        }
    }
}
