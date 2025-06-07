use avian3d::prelude::PhysicsLayer;

// Layers for physics colliders. Lets us filter out entities for doing spatial queries, otherwise we get fun stuff like boomerangs targeting the ground plane
// https://idanarye.github.io/bevy-tnua/avian3d/collision/collider/struct.CollisionLayers.html#creation
#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default, // Layer 0 - the default layer that all objects are assigned to
    Enemy,     // Layer 1
    Player,    // Layer 2
    Bullet,    // Layer 3
    Terrain,   // Layer 4
    Boomerang, // Layer 5
    DeadEnemy, // Layer 5
               //NoCollision, // Layer ?
}
