use crate::AppSystems::PreTickTimers;
use bevy::prelude::*;
use std::time::Duration;

pub fn plugin(app: &mut App) {
    app.init_resource::<DilatedTime>();
    app.add_systems(Update, scale_time.in_set(PreTickTimers));
}

/// Used by anything that needs to move in slow motion. We update this once per
/// frame, then everything with a velocity simply uses the delta from this
/// resource, rather that Res<Time>::delta
#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct DilatedTime {
    /// How much to scale the default timer by.
    /// E.g., at 0.5, our 60 Hz / ~16 ms/frame delta becomes ~8 ms
    pub scaling_factor: f32,

    /// Always use this to tick timers, etc. if something needs to be affected
    /// by slow motion. If not (like screen shake) use the regular game Res<Time>
    pub delta: Duration,
}

impl DilatedTime {
    /// The "minimum possible" speed time can go. We never fully pause the game during slo-mo.
    const SLOW_MO_SCALING_FACTOR: f32 = 0.1;
}

impl Default for DilatedTime {
    fn default() -> Self {
        Self {
            scaling_factor: 1.0,
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
