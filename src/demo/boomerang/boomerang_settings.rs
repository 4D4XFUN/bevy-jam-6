use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<BoomerangSettings>();

    // reflection
    app.register_type::<BoomerangSettings>();

}

#[cfg(feature = "dev")]
pub fn boomerang_dev_tools_plugin(app: &mut App) {
    use bevy_inspector_egui::quick::ResourceInspectorPlugin;
    app.add_plugins(ResourceInspectorPlugin::<BoomerangSettings>::default());
}

/// Current set of stats of our boomerang
#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct BoomerangSettings {
    pub movement_speed: f32,
    pub rotations_per_second: f32,
    pub falling_speed: f32,
    pub easing_function: EaseFunction,
}

impl Default for BoomerangSettings {
    fn default() -> Self {
        Self {
            movement_speed: 30.0,
            rotations_per_second: 12.0,
            falling_speed: 5.0,
            easing_function: EaseFunction::ElasticInOut,
        }
    }
}
