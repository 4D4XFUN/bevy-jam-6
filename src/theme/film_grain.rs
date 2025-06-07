// film_grain.rs
use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntry,
            BindingType, BufferInitDescriptor, BufferUsages,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
            MultisampleState, Operations, PipelineCache, PrimitiveState,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType,
            TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        view::{ExtractedView, ViewTarget},
        RenderApp,
    },
};

// Plugin to add the film grain effect
pub struct FilmGrainPlugin;

impl Plugin for FilmGrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<FilmGrainSettings>::default());

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
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct FilmGrainSettings {
    /// Intensity of the grain effect (0.0 - 1.0)
    pub grain_intensity: f32,
    /// Size of the grain particles
    pub grain_size: f32,
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
    // Padding for shader alignment
    _padding: f32,
}

impl Default for FilmGrainSettings {
    fn default() -> Self {
        Self {
            grain_intensity: 0.15,
            grain_size: 2.0,
            grain_speed: 1.0,
            tint_intensity: 0.1,
            vignette_intensity: 0.3,
            vignette_radius: 0.8,
            time: 0.0,
            _padding: 0.0,
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
            Err(_) => return Ok(()), // No settings, skip
        };

        let pipeline = match pipeline_cache.get_render_pipeline(film_grain_pipeline.pipeline_id) {
            Some(pipeline) => pipeline,
            None => return Ok(()), // Pipeline not ready
        };

        let post_process = view_target.post_process_write();

        let settings_uniform = render_context
            .render_device()
            .create_buffer_with_data(&bevy::render::render_resource::BufferInitDescriptor {
                label: Some("film_grain_settings"),
                contents: bytemuck::cast_slice(&[*settings]),
                usage: bevy::render::render_resource::BufferUsages::UNIFORM | bevy::render::render_resource::BufferUsages::COPY_DST,
            });

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
                            format: TextureFormat::bevy_default(),
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
pub fn update_film_grain_time(
    time: Res<Time>,
    mut query: Query<&mut FilmGrainSettings>,
) {
    for mut settings in &mut query {
        settings.time += time.delta_secs() * settings.grain_speed;
    }
}
