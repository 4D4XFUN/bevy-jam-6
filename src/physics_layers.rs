use avian3d::prelude::PhysicsLayer;

// Layers for physics colliders. Lets us filter out entities for doing spatial queries, otherwise we get fun stuff like boomerangs targeting the ground plane
// https://idanarye.github.io/bevy-tnua/avian3d/collision/collider/struct.CollisionLayers.html#creation
#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default, // Layer 0 - the default layer that all objects are assigned to
    Enemy, // Layer 1
    Terrain, // Layer 1
}

impl GameLayer {
    pub const ALL: [GameLayer; 2] = [GameLayer::Default, GameLayer::Enemy];
}
