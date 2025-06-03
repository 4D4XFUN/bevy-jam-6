use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct Velocity {
    value: Vec3,
}
