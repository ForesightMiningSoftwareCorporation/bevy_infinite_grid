use std::borrow::Cow;

use bevy::{
    asset::{load_internal_asset, weak_handle},
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    image::BevyDefault,
    math::FloatOrd,
    prelude::*,
    render::{
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, BlendState, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, DynamicUniformBuffer, FragmentState,
            MultisampleState, PipelineCache, PolygonMode, PrimitiveState, RenderPipelineDescriptor,
            ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
            StencilFaceState, StencilState, TextureFormat, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::RenderEntity,
        view::{ExtractedView, RenderVisibleEntities},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
};

use crate::InfiniteGrid2DSettings;

const GRID_2D_SHADER_HANDLE: Handle<Shader> = weak_handle!("01968ec1-1753-7731-9b47-b50296bcb87b");

pub fn render_app_builder_2d(app: &mut App) {
    load_internal_asset!(app, GRID_2D_SHADER_HANDLE, "grid_2d.wgsl", Shader::from_wgsl);

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<Grid2DViewUniforms>()
        .init_resource::<InfiniteGrid2DUniforms>()
        .init_resource::<InfiniteGrid2DPipeline>()
        .init_resource::<SpecializedRenderPipelines<InfiniteGrid2DPipeline>>()
        .add_render_command::<Transparent2d, DrawInfiniteGrid2D>()
        .add_systems(
            ExtractSchedule,
            (extract_infinite_grids_2d, extract_per_camera_settings_2d),
        )
        .add_systems(
            Render,
            (prepare_infinite_grids_2d, prepare_grid_2d_view_uniforms)
                .in_set(RenderSet::PrepareResources),
        )
        .add_systems(
            Render,
            (
                prepare_bind_groups_for_infinite_grids_2d,
                prepare_grid_2d_view_bind_groups,
            )
                .in_set(RenderSet::PrepareBindGroups),
        )
        .add_systems(Render, queue_infinite_grids_2d.in_set(RenderSet::Queue));
}

#[derive(Component)]
struct ExtractedInfiniteGrid2D {
    settings: InfiniteGrid2DSettings,
}

#[derive(Debug, ShaderType)]
pub struct InfiniteGrid2DUniform {
    scale: f32,
    x_axis_color: Vec3,
    y_axis_color: Vec3,
    minor_line_color: Vec4,
    major_line_color: Vec4,
}

impl InfiniteGrid2DUniform {
    fn from_settings(settings: &InfiniteGrid2DSettings) -> Self {
        Self {
            scale: settings.scale,
            x_axis_color: settings.x_axis_color.to_linear().to_vec3(),
            y_axis_color: settings.y_axis_color.to_linear().to_vec3(),
            minor_line_color: settings.minor_line_color.to_linear().to_vec4(),
            major_line_color: settings.major_line_color.to_linear().to_vec4(),
        }
    }
}

#[derive(Resource, Default)]
struct InfiniteGrid2DUniforms {
    uniforms: DynamicUniformBuffer<InfiniteGrid2DUniform>,
}

#[derive(Component)]
struct InfiniteGrid2DUniformOffset {
    offset: u32,
}

#[derive(Component)]
pub struct PerCameraSettings2DUniformOffset {
    offset: u32,
}

#[derive(Resource)]
struct InfiniteGrid2DBindGroup {
    value: BindGroup,
}

#[derive(Clone, ShaderType)]
pub struct Grid2DViewUniform {
    projection: Mat4,
    inverse_projection: Mat4,
    view: Mat4,
    inverse_view: Mat4,
    world_position: Vec3,
}

#[derive(Resource, Default)]
pub struct Grid2DViewUniforms {
    uniforms: DynamicUniformBuffer<Grid2DViewUniform>,
}

#[derive(Component)]
pub struct Grid2DViewUniformOffset {
    pub offset: u32,
}

#[derive(Component)]
struct Grid2DViewBindGroup {
    value: BindGroup,
}

struct SetGrid2DViewBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetGrid2DViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<Grid2DViewUniformOffset>, Read<Grid2DViewBindGroup>);
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, bind_group): ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.value, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}

struct SetInfiniteGrid2DBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetInfiniteGrid2DBindGroup<I> {
    type Param = SRes<InfiniteGrid2DBindGroup>;
    type ViewQuery = Option<Read<PerCameraSettings2DUniformOffset>>;
    type ItemQuery = Read<InfiniteGrid2DUniformOffset>;

    #[inline]
    fn render<'w>(
        _item: &P,
        camera_settings_offset: ROQueryItem<'w, Self::ViewQuery>,
        base_offset: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(base_offset) = base_offset else {
            warn!("InfiniteGrid2DUniformOffset missing");
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(
            I,
            &bind_group.into_inner().value,
            &[camera_settings_offset
                .map(|cs| cs.offset)
                .unwrap_or(base_offset.offset)],
        );
        RenderCommandResult::Success
    }
}

struct FinishDrawInfiniteGrid2D;

impl<P: PhaseItem> RenderCommand<P> for FinishDrawInfiniteGrid2D {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..4, 0..1);
        RenderCommandResult::Success
    }
}

fn prepare_grid_2d_view_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<Grid2DViewUniforms>,
    views: Query<(Entity, &ExtractedView)>,
) {
    view_uniforms.uniforms.clear();
    for (entity, camera) in views.iter() {
        let projection = camera.clip_from_view;
        let view = camera.world_from_view.compute_matrix();
        let inverse_view = view.inverse();
        commands.entity(entity).insert(Grid2DViewUniformOffset {
            offset: view_uniforms.uniforms.push(&Grid2DViewUniform {
                projection,
                view,
                inverse_view,
                inverse_projection: projection.inverse(),
                world_position: camera.world_from_view.translation(),
            }),
        });
    }

    view_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue)
}

fn prepare_grid_2d_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    uniforms: Res<Grid2DViewUniforms>,
    pipeline: Res<InfiniteGrid2DPipeline>,
    views: Query<Entity, With<Grid2DViewUniformOffset>>,
) {
    if let Some(binding) = uniforms.uniforms.binding() {
        for entity in views.iter() {
            let bind_group = render_device.create_bind_group(
                "grid-2d-view-bind-group",
                &pipeline.view_layout,
                &BindGroupEntries::single(binding.clone()),
            );
            commands
                .entity(entity)
                .insert(Grid2DViewBindGroup { value: bind_group });
        }
    }
}

fn extract_infinite_grids_2d(
    mut commands: Commands,
    grids: Extract<
        Query<(
            RenderEntity,
            &InfiniteGrid2DSettings,
            &RenderVisibleEntities,
        )>,
    >,
) {
    let extracted: Vec<_> = grids
        .iter()
        .map(|(entity, settings, visible_entities)| {
            (
                entity,
                (
                    ExtractedInfiniteGrid2D {
                        settings: *settings,
                    },
                    visible_entities.clone(),
                ),
            )
        })
        .collect();
    commands.try_insert_batch(extracted);
}

fn extract_per_camera_settings_2d(
    mut commands: Commands,
    cameras: Extract<Query<(RenderEntity, &InfiniteGrid2DSettings), With<Camera>>>,
) {
    let extracted: Vec<_> = cameras
        .iter()
        .map(|(entity, settings)| (entity, *settings))
        .collect();
    commands.try_insert_batch(extracted);
}

fn prepare_infinite_grids_2d(
    mut commands: Commands,
    grids: Query<(Entity, &ExtractedInfiniteGrid2D)>,
    cameras: Query<(Entity, &InfiniteGrid2DSettings), With<ExtractedView>>,
    mut uniforms: ResMut<InfiniteGrid2DUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    uniforms.uniforms.clear();
    for (entity, extracted) in &grids {
        commands.entity(entity).insert(InfiniteGrid2DUniformOffset {
            offset: uniforms
                .uniforms
                .push(&InfiniteGrid2DUniform::from_settings(&extracted.settings)),
        });
    }

    for (entity, settings) in &cameras {
        commands
            .entity(entity)
            .insert(PerCameraSettings2DUniformOffset {
                offset: uniforms
                    .uniforms
                    .push(&InfiniteGrid2DUniform::from_settings(settings)),
            });
    }

    uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

fn prepare_bind_groups_for_infinite_grids_2d(
    mut commands: Commands,
    uniforms: Res<InfiniteGrid2DUniforms>,
    pipeline: Res<InfiniteGrid2DPipeline>,
    render_device: Res<RenderDevice>,
) {
    let Some(binding) = uniforms.uniforms.binding() else {
        return;
    };

    let bind_group = render_device.create_bind_group(
        "infinite-grid-2d-bind-group",
        &pipeline.infinite_grid_layout,
        &BindGroupEntries::single(binding.clone()),
    );
    commands.insert_resource(InfiniteGrid2DBindGroup { value: bind_group });
}

#[allow(clippy::too_many_arguments)]
fn queue_infinite_grids_2d(
    pipeline_cache: Res<PipelineCache>,
    transparent_draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<InfiniteGrid2DPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<InfiniteGrid2DPipeline>>,
    infinite_grids: Query<&ExtractedInfiniteGrid2D>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa)>,
) {
    let draw_function_id = transparent_draw_functions
        .read()
        .get_id::<DrawInfiniteGrid2D>()
        .unwrap();

    for (view, entities, msaa) in views.iter_mut() {
        let Some(phase) = transparent_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            Grid2DPipelineKey {
                sample_count: msaa.samples(),
            },
        );
        for &entity in entities.iter::<InfiniteGrid2DSettings>() {
            if infinite_grids.get(entity.0).is_ok() {
                phase.items.push(Transparent2d {
                    pipeline: pipeline_id,
                    entity,
                    draw_function: draw_function_id,
                    sort_key: FloatOrd(f32::NEG_INFINITY),
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::None,
                    indexed: false,
                    extracted_index: 0,
                });
            }
        }
    }
}

type DrawInfiniteGrid2D = (
    SetItemPipeline,
    SetGrid2DViewBindGroup<0>,
    SetInfiniteGrid2DBindGroup<1>,
    FinishDrawInfiniteGrid2D,
);

#[derive(Resource)]
struct InfiniteGrid2DPipeline {
    view_layout: BindGroupLayout,
    infinite_grid_layout: BindGroupLayout,
}

impl FromWorld for InfiniteGrid2DPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let view_layout = render_device.create_bind_group_layout(
            "grid-2d-view-bind-group-layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                uniform_buffer::<Grid2DViewUniform>(true),
            ),
        );
        let infinite_grid_layout = render_device.create_bind_group_layout(
            "infinite-grid-2d-bind-group-layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                uniform_buffer::<InfiniteGrid2DUniform>(true),
            ),
        );

        Self {
            view_layout,
            infinite_grid_layout,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct Grid2DPipelineKey {
    sample_count: u32,
}

impl SpecializedRenderPipeline for InfiniteGrid2DPipeline {
    type Key = Grid2DPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed("grid-2d-render-pipeline")),
            layout: vec![self.view_layout.clone(), self.infinite_grid_layout.clone()],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: GRID_2D_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Cow::Borrowed("vertex"),
                buffers: vec![],
            },
            primitive: PrimitiveState {
                topology: bevy::render::mesh::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: bevy::render::render_resource::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: GRID_2D_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Cow::Borrowed("fragment"),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}