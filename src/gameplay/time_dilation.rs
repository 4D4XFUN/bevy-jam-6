use crate::AppSystems::PreTickTimers;
use bevy::prelude::*;
use std::time::Duration;

pub fn plugin(app: &mut App) {
    // init
    app.init_resource::<DilatedTime>();

    // systems
    app.add_systems(Update, scale_time.in_set(PreTickTimers));
    app.add_systems(Update, move_anything_with_a_velocity);
    app.add_systems(Update, rotate_anything_with_a_rotation);

    // reflection
    app.register_type::<VelocityDilated>();
    app.register_type::<DilatedTime>();
}

/// Used by anything that needs to move in slow motion. We update this once per
/// frame, then everything with a velocity simply uses the delta from this
/// resource, rather that `Res<Time>::delta`
#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct DilatedTime {
    /// How much to scale the default timer by.
    /// E.g., at 0.5, our 60 Hz / ~16 ms/frame delta becomes ~8 ms
    pub scaling_factor: f32,

    /// Always use this to tick timers, etc. if something needs to be affected
    /// by slow motion. If not (like screen shake) use the regular game `Res<Time>`
    pub delta: Duration,
}

impl DilatedTime {
    /// The "minimum possible" speed time can go. We never fully pause the game during slo-mo.
    pub const SLOW_MO_SCALING_FACTOR: f32 = 0.1;

    /// Intended to be drop-in replacement for `Res<Time>`
    pub fn delta(&self) -> Duration {
        self.delta
    }
    /// Intended to be drop-in replacement for `Res<Time>`
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }
}

impl Default for DilatedTime {
    fn default() -> Self {
        Self {
            scaling_factor: DilatedTime::SLOW_MO_SCALING_FACTOR,
            delta: Duration::from_secs(0),
        }
    }
}

fn scale_time(mut dilated_time_res: ResMut<DilatedTime>, actual_time: Res<Time>) -> Result {
    let scale = dilated_time_res.scaling_factor;
    let actual_delta = actual_time.delta();
    dilated_time_res.delta = actual_delta.mul_f32(scale);
    Ok(())
}

// ===============
// VELOCITY
// ===============
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct VelocityDilated(
    /// Represents how far the entity should travel in a second (and its heading).
    pub Vec3,
);

/// For anything with our special Velocity component and a transform, update the
/// transform by velocity - but make sure to scale by the time dilation factor
/// so that slow motion "just works"
fn move_anything_with_a_velocity(
    mut query: Query<(&VelocityDilated, &mut Transform)>,
    time: Res<DilatedTime>,
) {
    for (VelocityDilated(velocity), mut transform) in query.iter_mut() {
        transform.translation += velocity * time.delta_secs();
    }
}

// ===============
// ROTATION
// ===============
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct RotationDilated(
    /// Represents how fast the entity should rotate.
    pub f32,
);

fn rotate_anything_with_a_rotation(
    mut query: Query<(&RotationDilated, &mut Transform)>,
    time: Res<DilatedTime>,
) {
    for (RotationDilated(rotation_speed), mut transform) in query.iter_mut() {
        transform.rotate_local_y(rotation_speed * time.delta_secs());
    }
}
