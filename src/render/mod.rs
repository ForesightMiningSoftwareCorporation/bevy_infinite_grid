mod shadow;

use std::borrow::Cow;

use bevy::{
    core_pipeline::Transparent3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::PrimitiveTopology,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline,
        },
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
            BufferBindingType, BufferSize, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, DynamicUniformVec, FragmentState, MultisampleState,
            PipelineCache, PolygonMode, PrimitiveState, RenderPipelineDescriptor,
            SamplerBindingType, ShaderStages, SpecializedRenderPipeline,
            SpecializedRenderPipelines, StencilFaceState, StencilState, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, VisibleEntities},
        RenderApp, RenderStage,
    },
};

use crate::{GridFrustumIntersect, InfiniteGrid};

use shadow::{GridShadow, SetGridShadowBindGroup};

static PLANE_RENDER: &str = include_str!("plane_render.wgsl");

const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 15204473893972682982);

#[derive(Component)]
struct ExtractedInfiniteGrid {
    transform: GlobalTransform,
    grid: InfiniteGrid,
}

#[derive(Debug, AsStd140)]
pub struct InfiniteGridUniform {
    rot_matrix: Mat3,
    offset: Vec3,
    normal: Vec3,
    scale: f32,
    // 1 / fadeout_distance
    dist_fadeout_const: f32,

    dot_fadeout_const: f32,

    x_axis_color: Vec3,
    z_axis_color: Vec3,
    shadow_color: Vec4,
    minor_line_color: Vec4,
    major_line_color: Vec4,

    shadow_collapse_matrix: Mat3,
    shadow_center_pos: Vec3,
    shadow_texture_width: f32,
    shadow_texture_height: f32,
}

#[derive(Default)]
struct InfiniteGridUniforms {
    uniforms: DynamicUniformVec<InfiniteGridUniform>,
}

#[derive(Component)]
struct InfiniteGridUniformOffset {
    offset: u32,
}

struct InfiniteGridBindGroup {
    value: BindGroup,
}

#[derive(Clone, AsStd140)]
pub struct GridViewUniform {
    projection: Mat4,
    inverse_projection: Mat4,
    view: Mat4,
    inverse_view: Mat4,
    world_position: Vec3,
}

#[derive(Default)]
pub struct GridViewUniforms {
    uniforms: DynamicUniformVec<GridViewUniform>,
}

#[derive(Component)]
pub struct GridViewUniformOffset {
    pub offset: u32,
}

#[derive(Component)]
struct GridViewBindGroup {
    value: BindGroup,
}

struct SetGridViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetGridViewBindGroup<I> {
    type Param = SQuery<(Read<GridViewUniformOffset>, Read<GridViewBindGroup>)>;

    fn render<'w>(
        view: Entity,
        _item: Entity,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (view_uniform, bind_group) = param.get_inner(view).unwrap();
        pass.set_bind_group(I, &bind_group.value, &[view_uniform.offset]);

        RenderCommandResult::Success
    }
}

struct SetInfiniteGridBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetInfiniteGridBindGroup<I> {
    type Param = (
        SRes<InfiniteGridBindGroup>,
        SQuery<Read<InfiniteGridUniformOffset>>,
    );

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (bind_group, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let offset = query.get_inner(item).unwrap();
        pass.set_bind_group(I, &bind_group.into_inner().value, &[offset.offset]);
        RenderCommandResult::Success
    }
}

struct FinishDrawInfiniteGrid;

impl EntityRenderCommand for FinishDrawInfiniteGrid {
    type Param = ();

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..4, 0..1);
        RenderCommandResult::Success
    }
}

fn prepare_grid_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<GridViewUniforms>,
    views: Query<(Entity, &ExtractedView)>,
) {
    view_uniforms.uniforms.clear();
    for (entity, camera) in views.iter() {
        let projection = camera.projection;
        let view = camera.transform.compute_matrix();
        let inverse_view = view.inverse();
        commands.entity(entity).insert(GridViewUniformOffset {
            offset: view_uniforms.uniforms.push(GridViewUniform {
                projection,
                view,
                inverse_view,
                inverse_projection: projection.inverse(),
                world_position: camera.transform.translation,
            }),
        });
    }

    view_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue)
}

fn queue_grid_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    uniforms: Res<GridViewUniforms>,
    pipeline: Res<InfiniteGridPipeline>,
    views: Query<Entity, With<GridViewUniformOffset>>,
) {
    let binding = uniforms.uniforms.binding().unwrap();
    for entity in views.iter() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("grid-view-bind-group"),
            layout: &pipeline.view_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: binding.clone(),
            }],
        });
        commands
            .entity(entity)
            .insert(GridViewBindGroup { value: bind_group });
    }
}

fn extract_infinite_grids(
    mut commands: Commands,
    mut grids: Query<(
        Entity,
        &InfiniteGrid,
        &GlobalTransform,
        &GridFrustumIntersect,
        &mut VisibleEntities,
    )>,
) {
    for (entity, grid, transform, frustum_intersect, visible_entities) in grids.iter_mut() {
        commands.insert_or_spawn_batch(Some((
            entity,
            (
                ExtractedInfiniteGrid {
                    transform: transform.clone(),
                    grid: grid.clone(),
                },
                std::mem::take(visible_entities.into_inner()),
                *frustum_intersect,
                RenderPhase::<GridShadow>::default(),
            ),
        )));
    }
}

fn prepare_infinite_grids(
    mut commands: Commands,
    grids: Query<(Entity, &ExtractedInfiniteGrid, &GridFrustumIntersect)>,
    mut uniforms: ResMut<InfiniteGridUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    uniforms.uniforms.clear();
    for (entity, extracted, intersect) in grids.iter() {
        let transform = extracted.transform;
        let offset = transform.translation;
        let normal = transform.local_y();
        let rot_matrix = Mat3::from_quat(transform.rotation.inverse());
        commands.entity(entity).insert(InfiniteGridUniformOffset {
            offset: uniforms.uniforms.push(InfiniteGridUniform {
                rot_matrix,
                offset,
                normal,
                scale: transform.scale.x,
                dist_fadeout_const: 1. / extracted.grid.fadeout_distance,
                dot_fadeout_const: 1. / extracted.grid.dot_fadeout_strength,
                x_axis_color: Vec3::from_slice(&extracted.grid.x_axis_color.as_rgba_f32()),
                z_axis_color: Vec3::from_slice(&extracted.grid.z_axis_color.as_rgba_f32()),
                shadow_color: Vec4::from_slice(&extracted.grid.shadow_color.as_rgba_f32()),
                minor_line_color: Vec4::from_slice(&extracted.grid.minor_line_color.as_rgba_f32()),
                major_line_color: Vec4::from_slice(&extracted.grid.major_line_color.as_rgba_f32()),
                shadow_collapse_matrix: Mat3::from_cols(
                    normal.cross(-intersect.up_dir),
                    normal,
                    -intersect.up_dir,
                )
                .inverse(),
                shadow_center_pos: intersect.center,
                shadow_texture_height: intersect.height,
                shadow_texture_width: intersect.width,
            }),
        });
    }

    uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

fn queue_infinite_grids(
    mut pipeline_cache: ResMut<PipelineCache>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut commands: Commands,
    uniforms: Res<InfiniteGridUniforms>,
    pipeline: Res<InfiniteGridPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<InfiniteGridPipeline>>,
    render_device: Res<RenderDevice>,
    infinite_grids: Query<&ExtractedInfiniteGrid>,
    mut views: Query<(&VisibleEntities, &mut RenderPhase<Transparent3d>)>,
) {
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("infinite-grid-bind-group"),
        layout: &pipeline.infinite_grid_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniforms.uniforms.binding().unwrap(),
        }],
    });
    commands.insert_resource(InfiniteGridBindGroup { value: bind_group });

    let draw_function_id = transparent_draw_functions
        .read()
        .get_id::<DrawInfiniteGrid>()
        .unwrap();

    let pipeline = pipelines.specialize(&mut pipeline_cache, &pipeline, ());

    for (entities, mut phase) in views.iter_mut() {
        for &entity in &entities.entities {
            if infinite_grids.contains(entity) {
                phase.items.push(Transparent3d {
                    pipeline,
                    entity,
                    draw_function: draw_function_id,
                    distance: f32::NEG_INFINITY,
                });
            }
        }
    }
}

type DrawInfiniteGrid = (
    SetItemPipeline,
    SetGridViewBindGroup<0>,
    SetInfiniteGridBindGroup<1>,
    SetGridShadowBindGroup<2>,
    FinishDrawInfiniteGrid,
);

struct InfiniteGridPipeline {
    view_layout: BindGroupLayout,
    infinite_grid_layout: BindGroupLayout,
    grid_shadows_layout: BindGroupLayout,
}

impl FromWorld for InfiniteGridPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("grid-view-bind-group-layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(GridViewUniform::std140_size_static() as u64),
                },
                count: None,
            }],
        });
        let infinite_grid_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("infinite-grid-bind-group-layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(
                            InfiniteGridUniform::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let grid_shadows_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("grid-shadows-bind-group-layout"),
                entries: &[
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
                ],
            });

        Self {
            view_layout,
            infinite_grid_layout,
            grid_shadows_layout,
        }
    }
}

impl SpecializedRenderPipeline for InfiniteGridPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed("grid-render-pipeline")),
            layout: Some(vec![
                self.view_layout.clone(),
                self.infinite_grid_layout.clone(),
                self.grid_shadows_layout.clone(),
            ]),
            vertex: VertexState {
                shader: SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: Cow::Borrowed("vertex"),
                buffers: vec![],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: bevy::render::render_resource::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
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
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: Cow::Borrowed("fragment"),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }],
            }),
        }
    }
}

pub fn render_app_builder(app: &mut App) {
    app.world
        .resource_mut::<Assets<Shader>>()
        .set_untracked(SHADER_HANDLE, Shader::from_wgsl(PLANE_RENDER));

    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app
        .init_resource::<GridViewUniforms>()
        .init_resource::<InfiniteGridUniforms>()
        .init_resource::<InfiniteGridPipeline>()
        .init_resource::<SpecializedRenderPipelines<InfiniteGridPipeline>>()
        .add_render_command::<Transparent3d, DrawInfiniteGrid>()
        .add_system_to_stage(RenderStage::Extract, extract_infinite_grids)
        .add_system_to_stage(RenderStage::Prepare, prepare_infinite_grids)
        .add_system_to_stage(RenderStage::Prepare, prepare_grid_view_bind_groups)
        .add_system_to_stage(RenderStage::Queue, queue_infinite_grids)
        .add_system_to_stage(RenderStage::Queue, queue_grid_view_bind_groups);

    shadow::register_shadow(app);
}
