use bevy::prelude::*;

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct BoomerangAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

impl FromWorld for BoomerangAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        BoomerangAssets {
            mesh: asset_server.add(Mesh::from(Cuboid::new(1., 0.3, 0.3))),
            material: asset_server.add(StandardMaterial::from_color(Color::linear_rgb(
                0.6, 0.6, 0.0,
            ))),
        }
    }
}
