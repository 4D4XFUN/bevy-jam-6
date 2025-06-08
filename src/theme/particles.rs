use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::color::palettes::css;

pub fn plugin(app: &mut App) {
    app.init_resource::<SmokeParticleConfig>()
        .add_observer(spawn_gun_smoke)
        .add_systems(Update, update_smoke_particles)
        .add_plugins(MaterialPlugin::<SmokeMaterial>::default());
    
    // reflection
    app.register_type::<SmokeParticle>()
        .register_type::<SmokeParticleConfig>();

    // dev tool
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


// Custom material that always faces camera (billboard)
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SmokeMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub color: LinearRgba,
}

impl Material for SmokeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/smoke.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

// Spawn smoke as 3D quads
fn spawn_gun_smoke(
    trigger: Trigger<SpawnGunshotSmokeEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let event = trigger.event();

    // Create a quad mesh
    let quad_handle = meshes.add(Rectangle::new(1.0, 1.0));
    let texture_handle = asset_server.load("images/smoke_puff.png");

    // Spawn 3-5 smoke puffs
    for i in 0..4 {
        let random_offset = Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.2,
            (rand::random::<f32>() - 0.5) * 0.2,
            (rand::random::<f32>() - 0.5) * 0.2,
        );

        let velocity = event.direction * 2.0
            + Vec3::new(
            (rand::random::<f32>() - 0.5) * 1.0,
            rand::random::<f32>() * 0.5 + 0.5,
            (rand::random::<f32>() - 0.5) * 1.0,
        );

        let material = materials.add(SmokeMaterial {
            texture: texture_handle.clone(),
            color: LinearRgba::new(1.0, 1.0, 1.0, 0.7),
        });

        commands.spawn((
            Name::new("SmokeParticle"),
            Mesh3d(quad_handle.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(event.position + random_offset)
                .with_scale(Vec3::splat(0.5))
                .looking_at(event.position, Vec3::Y),
            SmokeParticle {
                velocity,
                lifetime: 0.0,
            },
            BillboardLock,
        ));
    }
}

// Component to mark billboards
#[derive(Component)]
struct BillboardLock;

// Update smoke particles
fn update_smoke_particles(
    mut commands: Commands,
    time: Res<Time>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<BillboardLock>)>,
    mut particles: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<SmokeMaterial>,
        &mut SmokeParticle,
    ), (With<BillboardLock>, Without<Camera3d>)>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    particle_config: Res<SmokeParticleConfig>,
) {
    let dt = time.delta_secs();

    // Get camera position for billboarding
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    for (entity, mut transform, material_handle, mut particle) in &mut particles {
        // Update lifetime
        particle.lifetime += dt;

        // Remove after max lifetime
        if particle.lifetime > particle_config.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Simple physics
        transform.translation += particle.velocity * dt;
        particle.velocity *= 0.95; // Drag
        particle.velocity.y += dt * 0.5; // Smoke rises

        // Billboard - make it face the camera
        transform.look_at(camera_transform.translation, Vec3::Y);

        // Grow
        let size = particle_config.tween_size(particle.lifetime);
        transform.scale = Vec3::splat(size);

        // Fade by updating material
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = 0.7 * (1.0 - particle.lifetime / particle_config.max_lifetime);
            material.color = LinearRgba::new(1.0, 1.0, 1.0, alpha);
        }
    }
}
