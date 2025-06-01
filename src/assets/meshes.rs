use bevy::prelude::*;

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct MeshAssets {
    pub boomerang: Handle<Mesh>,
}

impl FromWorld for MeshAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        MeshAssets {
            boomerang: asset_server.add(Mesh::from(Cuboid::new(0.2, 0.2, 0.2))),
        }
    }
}
