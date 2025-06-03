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
    pub min_movement_speed: f32,
    pub max_movement_speed: f32,
    pub min_rotation_speed: f32,
    pub max_rotation_speed: f32,
    pub falling_speed: f32,
    pub easing_function: EaseFunction, // see https://bevyengine.org/examples/animation/easing-functions/
}

impl Default for BoomerangSettings {
    fn default() -> Self {
        Self {
            min_movement_speed: 8.,
            max_movement_speed: 18.,
            min_rotation_speed: 10.,
            max_rotation_speed: 25.,
            falling_speed: 5.0,
            easing_function: EaseFunction::BackOut,
        }
    }
}

impl BoomerangSettings {
    pub fn tween_movement_speed(&self, progress: f32) -> f32 {
        self.tween_values(self.min_movement_speed, self.max_movement_speed, progress)
    }
    pub fn tween_rotation_speed(&self, progress: f32) -> f32 {
        self.tween_values(self.min_rotation_speed, self.max_rotation_speed, progress)
    }

    fn tween_values(&self, min: f32, max: f32, progress: f32) -> f32 {
        EasingCurve::new(min, max, self.easing_function)
            .sample(progress)
            .unwrap_or((min + max) / 2.)
    }
}
