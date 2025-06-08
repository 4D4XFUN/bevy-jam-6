use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<SmokeParticleConfig>()
        .add_observer(spawn_gun_smoke)
        .add_systems(Update, update_smoke_particles)
        .register_type::<SmokeParticle>()
        .register_type::<SmokeParticleConfig>();

    use bevy_inspector_egui::quick::ResourceInspectorPlugin;
    app.add_plugins(ResourceInspectorPlugin::<SmokeParticleConfig>::default());
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct SmokeParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
}

#[derive(Event, Debug, Copy, Clone)]
pub struct SpawnGunshotSmokeEvent {
    pub position: Vec3,
    pub direction: Vec3,
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct SmokeParticleConfig {
    lifetime: f32,
    max_lifetime: f32,
    min_size: f32,
    max_size: f32,
    ease_function: EaseFunction,
}
impl Default for SmokeParticleConfig {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 2.0,
            min_size: 5.0,
            max_size: 10.0,
            ease_function: EaseFunction::Linear,
        }
    }
}
impl SmokeParticleConfig {
    pub fn tween_size(&self, lifetime: f32) -> f32 {
        let progress = lifetime / self.max_lifetime;
        EasingCurve::new(self.min_size, self.max_size, self.ease_function).sample_clamped(progress)
    }
}

// Spawn a few smoke sprites
fn spawn_gun_smoke(
    trigger: Trigger<SpawnGunshotSmokeEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let event = trigger.event();

    // Spawn 3-5 smoke puffs
    for i in 0..4 {
        let random_offset = Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.2,
            (rand::random::<f32>() - 0.5) * 0.2,
            0.0,
        );

        let velocity = event.direction * 2.0
            + Vec3::new(
                (rand::random::<f32>() - 0.5) * 1.0,
                rand::random::<f32>() * 0.5 + 0.5, // Upward bias
                0.0,
            );

        commands
            .spawn((
                Name::new("SmokeParticles"),
                Sprite {
                    image: asset_server.load("images/smoke_puff.png"),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                    ..default()
                },
                Transform::from_translation(event.position + random_offset)
                    .with_scale(Vec3::splat(0.5)),
            ))
            .insert(SmokeParticle {
                velocity,
                lifetime: 0.0,
            });
    }
}

// Update smoke particles
fn update_smoke_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Transform, &mut Sprite, &mut SmokeParticle)>,
    particle_config: Res<SmokeParticleConfig>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut sprite, mut particle) in &mut particles {
        // Update lifetime
        particle.lifetime += dt;

        // Remove after 1 second
        if particle.lifetime > particle_config.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Simple physics
        transform.translation += particle.velocity * dt;
        particle.velocity *= 0.95; // Drag
        particle.velocity.y += dt * 0.5; // Smoke rises

        // Grow and fade
        let t = particle.lifetime;
        let size = particle_config.tween_size(t);
        transform.scale = Vec3::splat(size); // Grow from 0.5 to 1.0
        sprite.color.set_alpha(0.7 * (1.0 - t)); // Fade out
    }
}
