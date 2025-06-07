use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    reflect::Reflect,
    render::{
        RenderApp,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntry, BindingType,
            BufferInitDescriptor, BufferUsages, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FragmentState, MultisampleState, Operations, PipelineCache,
            PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        view::{ExtractedView, ViewTarget},
    },
};

// Plugin to add the film grain effect
pub struct FilmGrainPlugin;

impl Plugin for FilmGrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<FilmGrainSettings>::default(),
            FilmGrainSettingsTween::plugin,
        ))
            .register_type::<FilmGrainSettings>();

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_graph_node::<FilmGrainNode>(Core3d, FilmGrainLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    FilmGrainLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            )
            .add_render_graph_node::<FilmGrainNode>(Core2d, FilmGrainLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    FilmGrainLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<FilmGrainPipeline>();
    }
}

// Settings component that controls the effect
#[derive(
    Component, Clone, Copy, ExtractComponent, Reflect, ShaderType, bytemuck::Pod, bytemuck::Zeroable,
)]
#[reflect(Component)]
#[repr(C)]
pub struct FilmGrainSettings {
    /// Intensity of the grain effect (0.0 - 1.0)
    pub grain_intensity: f32,
    /// Grain scale - smaller values = finer grain (default: 0.004)
    pub grain_scale: f32,
    /// Speed of grain animation
    pub grain_speed: f32,
    /// Intensity of the yellow/sepia tint (0.0 - 1.0)
    pub tint_intensity: f32,
    /// Vignette intensity (0.0 - 1.0)
    pub vignette_intensity: f32,
    /// Vignette radius (0.0 - 1.0)
    pub vignette_radius: f32,
    /// Time for animation
    pub time: f32,
    /// Intensity of film artifacts (scratches, dust, hairs) (0.0 - 1.0)
    pub artifact_intensity: f32,
    /// How often scratches appear (0.0 = never, 1.0 = always)
    pub scratch_frequency: f32,
    /// How often dust specks appear (0.0 = never, 1.0 = always)
    pub dust_frequency: f32,
    /// How often hair fibers appear (0.0 = never, 1.0 = always)
    pub hair_frequency: f32,
    /// Padding for alignment
    _padding: f32,
}

impl Default for FilmGrainSettings {
    fn default() -> Self {
        Self {
            grain_intensity: 0.1,
            grain_scale: 0.004,
            grain_speed: 20.0,
            tint_intensity: 0.6,
            vignette_intensity: 1.0,
            vignette_radius: 0.7,
            time: 0.0,
            artifact_intensity: 0.7,
            scratch_frequency: 0.02,
            dust_frequency: 0.01,
            hair_frequency: 0.015,
            _padding: 0.0,
        }
    }
}

pub enum FilmGrainSettingsPresets {
    Default,
    VignetteClosed,
}
impl FilmGrainSettingsPresets {
    pub fn get(&self) -> FilmGrainSettings {
        match self {
            FilmGrainSettingsPresets::Default => FilmGrainSettings::default(),
            FilmGrainSettingsPresets::VignetteClosed => FilmGrainSettings {
                vignette_intensity: 1.0,
                vignette_radius: 0.0,
                ..default()
            },
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FilmGrainLabel;

// The render node that applies the effect
struct FilmGrainNode {
    query: QueryState<(&'static ViewTarget, &'static FilmGrainSettings), With<ExtractedView>>,
}

impl FromWorld for FilmGrainNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for FilmGrainNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph_context: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph_context.view_entity();
        let pipeline_cache = world.resource::<PipelineCache>();
        let film_grain_pipeline = world.resource::<FilmGrainPipeline>();

        let (view_target, settings) = match self.query.get_manual(world, view_entity) {
            Ok(result) => result,
            Err(_) => {
                // Debug: No settings found
                return Ok(());
            }
        };

        let pipeline = match pipeline_cache.get_render_pipeline(film_grain_pipeline.pipeline_id) {
            Some(pipeline) => pipeline,
            None => {
                // Debug: Pipeline not ready
                bevy::log::warn!("Film grain pipeline not ready yet");
                return Ok(());
            }
        };

        let post_process = view_target.post_process_write();

        let settings_uniform = render_context.render_device().create_buffer_with_data(
            &bevy::render::render_resource::BufferInitDescriptor {
                label: Some("film_grain_settings"),
                contents: bytemuck::cast_slice(&[*settings]),
                usage: bevy::render::render_resource::BufferUsages::UNIFORM
                    | bevy::render::render_resource::BufferUsages::COPY_DST,
            },
        );

        let bind_group = render_context.render_device().create_bind_group(
            "film_grain_bind_group",
            &film_grain_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &film_grain_pipeline.sampler,
                settings_uniform.as_entire_buffer_binding(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("film_grain_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

// Pipeline resource
#[derive(Resource)]
struct FilmGrainPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for FilmGrainPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "film_grain_bind_group_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(FilmGrainSettings::min_size()),
                    },
                    count: None,
                },
            ],
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/film_grain.wgsl");

        // Alternative: Try embedding the shader directly for testing
        // let shader = world.resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(
        //     include_str!("../assets/shaders/film_grain.wgsl"),
        //     "shaders/film_grain.wgsl"
        // ));

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("film_grain_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba16Float, // HDR format for post-processing
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

// Helper system to update time
pub fn update_film_grain_time(time: Res<Time>, mut query: Query<&mut FilmGrainSettings>) {
    for mut settings in &mut query {
        settings.time += time.delta_secs() * settings.grain_speed;
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FilmGrainSettingsTween {
    pub timer: Timer,
    pub ease_function: EaseFunction,
    _target: FilmGrainSettings,
    _original: Option<FilmGrainSettings>,
}

impl FilmGrainSettingsTween {
    pub fn new(
        seconds: f32,
        ease_function: EaseFunction,
        preset: FilmGrainSettingsPresets,
    ) -> Self {
        Self {
            timer: Timer::from_seconds(seconds, TimerMode::Once),
            ease_function,
            _target: FilmGrainSettingsPresets::get(&preset),
            _original: None,
        }
    }

    fn plugin(app: &mut App) {
        app.add_systems(Update, Self::update);
        app.register_type::<Self>();
    }

    fn tween<F>(&self, extractor: F) -> Option<f32>
    where
        F: Fn(&FilmGrainSettings) -> f32,
    {
        if let Some(original) = &self._original {
            let progress = self.timer.fraction();
            EasingCurve::new(
                extractor(original),
                extractor(&self._target),
                self.ease_function,
            )
                .sample(progress)
        } else {
            None
        }
    }

    pub fn update(
        mut query: Query<(&mut FilmGrainSettings, &mut FilmGrainSettingsTween)>,
        time: Res<Time>,
        mut commands: Commands,
    ) {
        for (mut settings, mut settings_tween) in query.iter_mut() {
            // tick the timer
            settings_tween.timer.tick(time.delta());

            // save the original if we haven't already
            if settings_tween._original.is_none() {
                settings_tween._original = Some(settings.clone());
            }

            // sample our easing function
            let progress = settings_tween.timer.fraction();
            let f = settings_tween.ease_function;
            let tween = f.sample_clamped(progress);

            // interpolate all the values
            settings_tween
                .tween(|s| s.vignette_radius)
                .map(|new_val| settings.vignette_radius = new_val);
            settings_tween
                .tween(|s| s.vignette_intensity)
                .map(|new_val| settings.vignette_intensity = new_val);
            // todo do the rest if we need them
        }
    }

    fn cleanup(query: Query<(Entity,  &FilmGrainSettingsTween )>, mut commands: Commands) {
        for (e, f) in query {
            if f.timer.finished() {
                commands.entity(e).remove::<FilmGrainSettingsTween>();
            }
        }
    }
}
