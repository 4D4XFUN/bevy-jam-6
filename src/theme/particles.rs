use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::color::palettes::css;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};

pub fn plugin(app: &mut App) {
    app.init_resource::<SmokeParticleConfig>()
        .add_observer(spawn_gun_smoke)
        .add_systems(Update, update_smoke_particles);

    // reflection
    app.register_type::<SmokeParticle>()
        .register_type::<SmokeParticleConfig>();

    // dev tool
    // use bevy_inspector_egui::quick::ResourceInspectorPlugin;
    // app.add_plugins(ResourceInspectorPlugin::<SmokeParticleConfig>::default());
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
    particles_per_shot: usize,
    particle_spread: f32,
}
impl Default for SmokeParticleConfig {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 2.0,
            min_size: 0.5,
            max_size: 1.0,
            ease_function: EaseFunction::ExponentialOut,
            particles_per_shot: 15,
            particle_spread: 1.0,
        }
    }
}
impl SmokeParticleConfig {
    pub fn tween_size(&self, lifetime: f32) -> f32 {
        let progress = lifetime / self.max_lifetime;
        EasingCurve::new(self.min_size, self.max_size, self.ease_function).sample_clamped(progress)
    }
}

fn spawn_gun_smoke(
    trigger: Trigger<SpawnGunshotSmokeEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    particle_configs: Res<SmokeParticleConfig>,
) {
    let event = trigger.event();

    let quad_handle = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));

    for i in 0..particle_configs.particles_per_shot {
        let random_offset = Vec3::new(
            (rand::random::<f32>() - 0.5) * particle_configs.particle_spread,
            (rand::random::<f32>() - 0.5) * particle_configs.particle_spread,
            (rand::random::<f32>() - 0.5) * particle_configs.particle_spread,
        );

        let velocity = event.direction * 2.0
            + Vec3::new(
            (rand::random::<f32>() - 0.5) * 1.0,
            rand::random::<f32>() * 0.5 + 0.5,
            (rand::random::<f32>() - 0.5) * 1.0,
        );

        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            ..default()
        });

        commands.spawn((
            Name::new("SmokeParticle"),
            Mesh3d(quad_handle.clone()),
            MeshMaterial3d(material),
            // Transform::from_translation(event.position + random_offset)
            //     .with_scale(Vec3::splat(0.5)),
            Transform::from_translation(event.position + random_offset)
                .with_scale(Vec3::splat(2.0))
                .looking_at(event.position + event.direction, Vec3::Y),
            SmokeParticle {
                velocity,
                lifetime: 0.0,
            },
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}

fn update_smoke_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut SmokeParticle,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    particle_config: Res<SmokeParticleConfig>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, material_handle, mut particle) in &mut particles {
        particle.lifetime += dt;

        if particle.lifetime > particle_config.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Move particles
        transform.translation += particle.velocity * dt;
        particle.velocity *= 0.95;
        particle.velocity.y += dt * 0.5;

        // Scale over time
        let size = particle_config.tween_size(particle.lifetime);
        transform.scale = Vec3::splat(size);

        // Fade out linearly over time
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = 0.7 * (1.0 - particle.lifetime / particle_config.max_lifetime);
            material.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
    }
}
