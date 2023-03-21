mod shadow;

pub use shadow::RenderSettings;

use std::borrow::Cow;

use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        }
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::PrimitiveTopology,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
            BufferBindingType, BufferSize, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, DynamicUniformBuffer, FragmentState,
            MultisampleState, PipelineCache, PolygonMode, PrimitiveState, RenderPipelineDescriptor,
            SamplerBindingType, ShaderStages, ShaderType, SpecializedRenderPipeline,
            SpecializedRenderPipelines, StencilFaceState, StencilState, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, VisibleEntities},
        RenderSet, Extract, ExtractSchedule, RenderApp,
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

#[derive(Debug, ShaderType)]
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
    minor_line_color: Vec4,
    major_line_color: Vec4,
}

#[derive(Debug, ShaderType)]
pub struct GridShadowUniform {
    shadow_color: Vec4,
    shadow_collapse_matrix: Mat3,
    shadow_center_pos: Vec3,
    shadow_texture_width: f32,
    shadow_texture_height: f32,
}

#[derive(Resource, Default)]
struct InfiniteGridUniforms {
    uniforms: DynamicUniformBuffer<InfiniteGridUniform>,
}

#[derive(Resource, Default)]
struct GridShadowUniforms {
    uniforms: DynamicUniformBuffer<GridShadowUniform>,
}

#[derive(Component)]
struct InfiniteGridUniformOffset {
    offset: u32,
}

#[derive(Component)]
pub struct GridShadowUniformOffset {
    offset: u32,
}

#[derive(Resource)]
struct InfiniteGridBindGroup {
    value: BindGroup,
}

#[derive(Clone, ShaderType)]
pub struct GridViewUniform {
    projection: Mat4,
    inverse_projection: Mat4,
    view: Mat4,
    inverse_view: Mat4,
    world_position: Vec3,
}

#[derive(Resource, Default)]
pub struct GridViewUniforms {
    uniforms: DynamicUniformBuffer<GridViewUniform>,
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

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetGridViewBindGroup<I> {
    type Param = ();
    type ViewWorldQuery = (Read<GridViewUniformOffset>, Read<GridViewBindGroup>);
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, bind_group): ROQueryItem<'w, Self::ViewWorldQuery>,
        _entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.value, &[view_uniform.offset]);

        RenderCommandResult::Success
    }
}

struct SetInfiniteGridBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetInfiniteGridBindGroup<I> {
    type Param = SRes<InfiniteGridBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<InfiniteGridUniformOffset>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        offset: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.into_inner().value, &[offset.offset]);
        RenderCommandResult::Success
    }
}

struct FinishDrawInfiniteGrid;

impl<P: PhaseItem> RenderCommand<P> for FinishDrawInfiniteGrid {
    type Param = ();
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        _entity: ROQueryItem<'w, Self::ItemWorldQuery>,
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
                world_position: camera.transform.translation(),
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
    if let Some(binding) = uniforms.uniforms.binding() {
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
}

fn extract_infinite_grids(
    mut commands: Commands,
    grids: Extract<Query<(Entity, &InfiniteGrid, &GlobalTransform, &VisibleEntities)>>,
) {
    let extracted: Vec<_> = grids
        .iter()
        .map(|(entity, grid, transform, visible_entities)| {
            (
                entity,
                (
                    ExtractedInfiniteGrid {
                        transform: transform.clone(),
                        grid: grid.clone(),
                    },
                    visible_entities.clone(),
                    RenderPhase::<GridShadow>::default(),
                ),
            )
        })
        .collect();
    commands.insert_or_spawn_batch(extracted);
}

fn extract_grid_shadows(
    mut commands: Commands,
    grids: Extract<Query<(Entity, &ExtractedInfiniteGrid, &GridFrustumIntersect)>>,
) {
    let extracted: Vec<_> = grids
        .iter()
        .filter(|(_, extracted, _)| extracted.grid.shadow_color.is_some())
        .map(|(entity, _, intersect)| (entity, (intersect.clone(),)))
        .collect();
    commands.insert_or_spawn_batch(extracted);
}

fn prepare_infinite_grids(
    mut commands: Commands,
    grids: Query<(Entity, &ExtractedInfiniteGrid)>,
    mut uniforms: ResMut<InfiniteGridUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    uniforms.uniforms.clear();
    for (entity, extracted) in grids.iter() {
        let transform = extracted.transform;
        let t = transform.compute_transform();
        let offset = transform.translation();
        let normal = transform.up();
        let rot_matrix = Mat3::from_quat(t.rotation.inverse());
        commands.entity(entity).insert(InfiniteGridUniformOffset {
            offset: uniforms.uniforms.push(InfiniteGridUniform {
                rot_matrix,
                offset,
                normal,
                scale: t.scale.x,
                dist_fadeout_const: 1. / extracted.grid.fadeout_distance,
                dot_fadeout_const: 1. / extracted.grid.dot_fadeout_strength,
                x_axis_color: Vec3::from_slice(&extracted.grid.x_axis_color.as_rgba_f32()),
                z_axis_color: Vec3::from_slice(&extracted.grid.z_axis_color.as_rgba_f32()),
                minor_line_color: Vec4::from_slice(&extracted.grid.minor_line_color.as_rgba_f32()),
                major_line_color: Vec4::from_slice(&extracted.grid.major_line_color.as_rgba_f32()),
            }),
        });
    }

    uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

fn prepare_grid_shadows(
    mut commands: Commands,
    grids: Query<(Entity, &ExtractedInfiniteGrid, &GridFrustumIntersect)>,
    mut uniforms: ResMut<GridShadowUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    uniforms.uniforms.clear();
    for (entity, extracted, intersect) in grids.iter() {
        let transform = extracted.transform;
        let normal = transform.up();

        // When called after [`extract_grid_shadows()`] has filtered out
        // [`InfiniteGrid`]s that have shadow_color: None, this is always
        // true. However, if this is ever called before then the unwrap()
        // that was here before will crash the program with a panic! that
        // makes shadow_color: None unusable.
        if let Some(grid_shadow_color) = extracted.grid.shadow_color {
            commands.entity(entity).insert(GridShadowUniformOffset {
                offset: uniforms.uniforms.push(GridShadowUniform {
                    shadow_color: Vec4::from_slice(&grid_shadow_color.as_rgba_f32()),
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
    intersects: Query<&GridFrustumIntersect>,
    mut views: Query<(
        &VisibleEntities,
        &mut RenderPhase<Transparent3d>,
        &ExtractedView,
    )>,
) {
    let bind_group = if let Some(binding) = uniforms.uniforms.binding() {
        render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("infinite-grid-bind-group"),
            layout: &pipeline.infinite_grid_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: binding.clone(),
            }],
        })
    } else {
        return;
    };
    commands.insert_resource(InfiniteGridBindGroup { value: bind_group });

    let draw_function_id = transparent_draw_functions
        .read()
        .get_id::<DrawInfiniteGrid>()
        .unwrap();

    let base_pipeline = pipelines.specialize(
        &mut pipeline_cache,
        &pipeline,
        GridPipelineKey { has_shadows: false },
    );
    let shadow_pipeline = pipelines.specialize(
        &mut pipeline_cache,
        &pipeline,
        GridPipelineKey { has_shadows: true },
    );

    for (entities, mut phase, view) in views.iter_mut() {
        for &entity in &entities.entities {
            if infinite_grids
                .get(entity)
                .map(|grid| plane_check(&grid.transform, view.transform.translation()))
                .unwrap_or(false)
            {
                phase.items.push(Transparent3d {
                    pipeline: match intersects.contains(entity) {
                        true => shadow_pipeline,
                        false => base_pipeline,
                    },
                    entity,
                    draw_function: draw_function_id,
                    distance: f32::NEG_INFINITY,
                });
            }
        }
    }
}

fn plane_check(plane: &GlobalTransform, point: Vec3) -> bool {
    plane.up().dot(plane.translation() - point).abs() > f32::EPSILON
}

type DrawInfiniteGrid = (
    SetItemPipeline,
    SetGridViewBindGroup<0>,
    SetInfiniteGridBindGroup<1>,
    SetGridShadowBindGroup<2>,
    FinishDrawInfiniteGrid,
);

#[derive(Resource)]
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
                    min_binding_size: BufferSize::new(GridViewUniform::min_size().into()),
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
                        min_binding_size: BufferSize::new(InfiniteGridUniform::min_size().into()),
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
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: BufferSize::new(GridShadowUniform::min_size().into()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
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

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct GridPipelineKey {
    has_shadows: bool,
}

impl SpecializedRenderPipeline for InfiniteGridPipeline {
    type Key = GridPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed(
                key.has_shadows
                    .then(|| "grid-render-pipeline")
                    .unwrap_or("grid-render-pipeline-shadowless"),
            )),
            layout: [self.view_layout.clone(), self.infinite_grid_layout.clone()]
                .into_iter()
                .chain(key.has_shadows.then(|| self.grid_shadows_layout.clone()))
                .collect(),
            push_constant_ranges: Vec::new(),
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
                shader_defs: key
                    .has_shadows
                    .then(|| "SHADOWS".into())
                    .into_iter()
                    .collect(),
                entry_point: Cow::Borrowed("fragment"),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
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
        .init_resource::<GridShadowUniforms>()
        .init_resource::<InfiniteGridPipeline>()
        .init_resource::<SpecializedRenderPipelines<InfiniteGridPipeline>>()
        .add_render_command::<Transparent3d, DrawInfiniteGrid>()
        .add_system(extract_infinite_grids.in_schedule(ExtractSchedule))
        .add_system(extract_grid_shadows
            .in_schedule(ExtractSchedule)
            .before(extract_infinite_grids) // order to minimize move overhead
        )
        .add_system(prepare_infinite_grids.in_set(RenderSet::Prepare))
        .add_system(prepare_grid_shadows.in_set(RenderSet::Prepare))
        .add_system(prepare_grid_view_bind_groups.in_set(RenderSet::Prepare))
        .add_system(queue_infinite_grids.in_set(RenderSet::Queue))
        .add_system(queue_grid_view_bind_groups.in_set(RenderSet::Queue));

    shadow::register_shadow(app);
}
