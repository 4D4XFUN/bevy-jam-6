use avian3d::prelude::{LinearVelocity, Physics};
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, spawn_footsteps);
}

#[derive(Resource)]
struct FootstepEffect(Handle<EffectAsset>);

fn setup(mut effects: ResMut<Assets<EffectAsset>>, mut commands: Commands) {
    // Define a color gradient from red to transparent black
    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(0.5, 0.5, 0.5, 1.0));
    gradient.add_key(1.0, Vec4::new(0.5, 0.5, 0.5,0.0));

    // Create a new expression module
    let mut module = Module::default();

    // On spawn, randomly initialize the position of the particle
    // to be over the surface of a sphere of radius 0.1 units.
    let init_pos = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(0.1),
        dimension: ShapeDimension::Surface,
    };

    let init_size = SetSizeModifier {
        size: CpuValue::Single(Vec3::splat(0.3))
    };

    // Also initialize a radial initial velocity to 6 units/sec
    // away from the (same) sphere center.
    let init_vel = SetVelocityTangentModifier {
        origin: module.lit(Vec3::ZERO),
        axis: module.lit(Vec3::new(0.0, 1.0, -1.0)),
        speed: module.lit(5.0),
    };

    // Initialize the total lifetime of the particle, that is
    // the time for which it's simulated and rendered. This modifier
    // is almost always required, otherwise the particles won't show.
    let lifetime = module.lit(0.5); // literal value "10.0"
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Every frame, add a gravity-like acceleration downward
    let accel = module.lit(Vec3::new(0., -3., 0.));
    let update_accel = AccelModifier::new(accel);

    // Create the effect asset
    let effect = EffectAsset::new(
        // Maximum number of particles alive at a time
        32768,
        // Spawn at a rate of 5 particles per second
        SpawnerSettings::once(10.0.into()),
        // Move the expression module into the asset
        module,
    )
    .with_name("FootstepEffect")
    .init(init_pos)
    .init(init_vel)
    .init(init_lifetime)
    .update(update_accel)
    // Render the particles with a color gradient over their
    // lifetime. This maps the gradient key 0 to the particle spawn
    // time, and the gradient key 1 to the particle death (10s).
    .render(ColorOverLifetimeModifier {
        gradient,
        ..default()
    })
    .render(init_size);

    // Insert into the asset system
    let effect_handle = effects.add(effect);

    commands.insert_resource(FootstepEffect(effect_handle));
}

#[derive(Component, Deref, DerefMut)]
pub struct Footsteps(Timer);

impl Footsteps {
    fn elapse(&mut self) {
        self.0.tick(self.0.duration());
    }
}

impl Default for Footsteps {
    fn default() -> Self {
        Footsteps(Timer::from_seconds(0.5, TimerMode::Repeating))
    }
}

fn spawn_footsteps(
    time: Res<Time<Physics>>,
    mut query: Query<(&Transform, &mut Footsteps, &LinearVelocity)>,
    footstep_effect: Res<FootstepEffect>,
    mut commands: Commands,
) {
    for (transform, mut footsteps, linear_velocity) in &mut query {
        if linear_velocity.0 != Vec3::ZERO {
            //moving
            footsteps.tick(time.delta());
            if footsteps.finished() {
                commands.spawn((
                    ParticleEffect::new(footstep_effect.0.clone()),
                    Transform::from_translation(transform.translation).looking_at(transform.translation + linear_velocity.0, Vec3::Y),
                ));
            }
        } else {
            footsteps.elapse();
        }
    }
}
