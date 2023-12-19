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
        },
    },
    pbr::MeshPipelineKey,
    prelude::*,
    render::{
        mesh::PrimitiveTopology,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline,
        },
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType, BufferSize,
            ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
            DynamicUniformBuffer, FragmentState, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, RenderPipelineDescriptor, SamplerBindingType, ShaderStages, ShaderType,
            SpecializedRenderPipeline, SpecializedRenderPipelines, StencilFaceState, StencilState,
            TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget, VisibleEntities},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
};

use crate::{GridFrustumIntersect, InfiniteGridSettings};

use shadow::{GridShadow, SetGridShadowBindGroup};

static PLANE_RENDER: &str = include_str!("plane_render.wgsl");

const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(15204473893972682982);

#[derive(Component)]
struct ExtractedInfiniteGrid {
    transform: GlobalTransform,
    grid: InfiniteGridSettings,
}

#[derive(Debug, ShaderType)]
pub struct InfiniteGridUniform {
    rot_matrix: Mat3,
    offset: Vec3,
    normal: Vec3,
}

#[derive(Debug, ShaderType)]
pub struct GridDisplaySettingsUniform {
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
struct GridDisplaySettingsUniforms {
    uniforms: DynamicUniformBuffer<GridDisplaySettingsUniform>,
}

#[derive(Resource, Default)]
struct GridShadowUniforms {
    uniforms: DynamicUniformBuffer<GridShadowUniform>,
}

#[derive(Component)]
struct InfiniteGridUniformOffsets {
    position_offset: u32,
    settings_offset: u32,
}

#[derive(Component)]
pub struct GridShadowUniformOffset {
    offset: u32,
}

#[derive(Component)]
pub struct PerCameraSettingsUniformOffset {
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
    type ViewWorldQuery = Option<Read<PerCameraSettingsUniformOffset>>;
    type ItemWorldQuery = Read<InfiniteGridUniformOffsets>;

    #[inline]
    fn render<'w>(
        _item: &P,
        camera_settings_offset: ROQueryItem<'w, Self::ViewWorldQuery>,
        base_offsets: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            &bind_group.into_inner().value,
            &[
                base_offsets.position_offset,
                camera_settings_offset
                    .map(|cs| cs.offset)
                    .unwrap_or(base_offsets.settings_offset),
            ],
        );
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

fn prepare_grid_view_uniforms(
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

fn prepare_grid_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    uniforms: Res<GridViewUniforms>,
    pipeline: Res<InfiniteGridPipeline>,
    views: Query<Entity, With<GridViewUniformOffset>>,
) {
    if let Some(binding) = uniforms.uniforms.binding() {
        for entity in views.iter() {
            let bind_group = render_device.create_bind_group(
                "grid-view-bind-group",
                &pipeline.view_layout,
                &BindGroupEntries::single(binding.clone()),
            );
            commands
                .entity(entity)
                .insert(GridViewBindGroup { value: bind_group });
        }
    }
}

fn extract_infinite_grids(
    mut commands: Commands,
    grids: Extract<
        Query<(
            Entity,
            &InfiniteGridSettings,
            &GlobalTransform,
            &VisibleEntities,
        )>,
    >,
) {
    let extracted: Vec<_> = grids
        .iter()
        .map(|(entity, grid, transform, visible_entities)| {
            (
                entity,
                (
                    ExtractedInfiniteGrid {
                        transform: *transform,
                        grid: *grid,
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
    grids: Extract<Query<(Entity, &InfiniteGridSettings, &GridFrustumIntersect)>>,
) {
    let extracted: Vec<_> = grids
        .iter()
        .filter(|(_, grid_settings, _)| grid_settings.shadow_color.is_some())
        .map(|(entity, _, intersect)| (entity, (*intersect,)))
        .collect();
    commands.insert_or_spawn_batch(extracted);
}

fn extract_per_camera_settings(
    mut commands: Commands,
    cameras: Extract<Query<(Entity, &InfiniteGridSettings), With<Camera>>>,
) {
    let extracted: Vec<_> = cameras
        .iter()
        .map(|(entity, settings)| (entity, *settings))
        .collect();
    commands.insert_or_spawn_batch(extracted);
}

fn prepare_infinite_grids(
    mut commands: Commands,
    grids: Query<(Entity, &ExtractedInfiniteGrid)>,
    cameras: Query<(Entity, &InfiniteGridSettings), With<ExtractedView>>,
    mut position_uniforms: ResMut<InfiniteGridUniforms>,
    mut settings_uniforms: ResMut<GridDisplaySettingsUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    position_uniforms.uniforms.clear();
    for (entity, extracted) in grids.iter() {
        let transform = extracted.transform;
        let t = transform.compute_transform();
        let offset = transform.translation();
        let normal = transform.up();
        let rot_matrix = Mat3::from_quat(t.rotation.inverse());
        commands.entity(entity).insert(InfiniteGridUniformOffsets {
            position_offset: position_uniforms.uniforms.push(InfiniteGridUniform {
                rot_matrix,
                offset,
                normal,
            }),
            settings_offset: settings_uniforms.uniforms.push(GridDisplaySettingsUniform {
                scale: extracted.grid.scale,
                dist_fadeout_const: 1. / extracted.grid.fadeout_distance,
                dot_fadeout_const: 1. / extracted.grid.dot_fadeout_strength,
                x_axis_color: Vec3::from_slice(&extracted.grid.x_axis_color.as_rgba_f32()),
                z_axis_color: Vec3::from_slice(&extracted.grid.z_axis_color.as_rgba_f32()),
                minor_line_color: Vec4::from_slice(&extracted.grid.minor_line_color.as_rgba_f32()),
                major_line_color: Vec4::from_slice(&extracted.grid.major_line_color.as_rgba_f32()),
            }),
        });
    }

    for (entity, settings) in cameras.iter() {
        commands
            .entity(entity)
            .insert(PerCameraSettingsUniformOffset {
                offset: settings_uniforms.uniforms.push(GridDisplaySettingsUniform {
                    scale: settings.scale,
                    dist_fadeout_const: 1. / settings.fadeout_distance,
                    dot_fadeout_const: 1. / settings.dot_fadeout_strength,
                    x_axis_color: Vec3::from_slice(&settings.x_axis_color.as_rgba_f32()),
                    z_axis_color: Vec3::from_slice(&settings.z_axis_color.as_rgba_f32()),
                    minor_line_color: Vec4::from_slice(&settings.minor_line_color.as_rgba_f32()),
                    major_line_color: Vec4::from_slice(&settings.major_line_color.as_rgba_f32()),
                }),
            });
    }

    position_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);

    settings_uniforms
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

fn prepare_bind_groups_for_infinite_grids(
    mut commands: Commands,
    position_uniforms: Res<InfiniteGridUniforms>,
    settings_uniforms: Res<GridDisplaySettingsUniforms>,
    pipeline: Res<InfiniteGridPipeline>,
    render_device: Res<RenderDevice>,
) {
    let bind_group = if let Some((position_binding, settings_binding)) = position_uniforms
        .uniforms
        .binding()
        .zip(settings_uniforms.uniforms.binding())
    {
        render_device.create_bind_group(
            "infinite-grid-bind-group",
            &pipeline.infinite_grid_layout,
            &BindGroupEntries::sequential((position_binding.clone(), settings_binding.clone())),
        )
    } else {
        return;
    };
    commands.insert_resource(InfiniteGridBindGroup { value: bind_group });
}

#[allow(clippy::too_many_arguments)]
fn queue_infinite_grids(
    pipeline_cache: Res<PipelineCache>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<InfiniteGridPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<InfiniteGridPipeline>>,
    infinite_grids: Query<&ExtractedInfiniteGrid>,
    intersects: Query<&GridFrustumIntersect>,
    mut views: Query<(
        &VisibleEntities,
        &mut RenderPhase<Transparent3d>,
        &ExtractedView,
    )>,
    msaa: Res<Msaa>,
) {
    let draw_function_id = transparent_draw_functions
        .read()
        .get_id::<DrawInfiniteGrid>()
        .unwrap();

    for (entities, mut phase, view) in views.iter_mut() {
        let mesh_key = MeshPipelineKey::from_hdr(view.hdr);
        let base_pipeline = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            GridPipelineKey {
                mesh_key,
                has_shadows: false,
                sample_count: msaa.samples(),
            },
        );
        let shadow_pipeline = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            GridPipelineKey {
                mesh_key,
                has_shadows: true,
                sample_count: msaa.samples(),
            },
        );
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
                    batch_range: 0..1,
                    dynamic_offset: None,
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
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: BufferSize::new(
                                InfiniteGridUniform::min_size().into(),
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: BufferSize::new(
                                GridDisplaySettingsUniform::min_size().into(),
                            ),
                        },
                        count: None,
                    },
                ],
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
    mesh_key: MeshPipelineKey,
    has_shadows: bool,
    sample_count: u32,
}

impl SpecializedRenderPipeline for InfiniteGridPipeline {
    type Key = GridPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = match key.mesh_key.contains(MeshPipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed(if key.has_shadows {
                "grid-render-pipeline"
            } else {
                "grid-render-pipeline-shadowless"
            })),
            layout: [self.view_layout.clone(), self.infinite_grid_layout.clone()]
                .into_iter()
                .chain(key.has_shadows.then(|| self.grid_shadows_layout.clone()))
                .collect(),
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: SHADER_HANDLE,
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
                depth_write_enabled: false,
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
                count: key.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: SHADER_HANDLE,
                shader_defs: key
                    .has_shadows
                    .then(|| "SHADOWS".into())
                    .into_iter()
                    .collect(),
                entry_point: Cow::Borrowed("fragment"),
                targets: vec![Some(ColorTargetState {
                    format,
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
        .get_or_insert_with(SHADER_HANDLE, || Shader::from_wgsl(PLANE_RENDER, file!()));

    let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<GridViewUniforms>()
        .init_resource::<InfiniteGridUniforms>()
        .init_resource::<GridDisplaySettingsUniforms>()
        .init_resource::<GridShadowUniforms>()
        .init_resource::<InfiniteGridPipeline>()
        .init_resource::<SpecializedRenderPipelines<InfiniteGridPipeline>>()
        .add_render_command::<Transparent3d, DrawInfiniteGrid>()
        .add_systems(
            ExtractSchedule,
            (extract_grid_shadows, extract_infinite_grids).chain(), // order to minimize move overhead
        )
        .add_systems(ExtractSchedule, extract_per_camera_settings)
        .add_systems(
            Render,
            (
                prepare_infinite_grids,
                prepare_grid_shadows,
                prepare_grid_view_uniforms,
            )
                .in_set(RenderSet::Prepare),
        )
        .add_systems(
            Render,
            (
                prepare_bind_groups_for_infinite_grids,
                prepare_grid_view_bind_groups,
            )
                .in_set(RenderSet::PrepareBindGroups),
        )
        .add_systems(Render, queue_infinite_grids.in_set(RenderSet::Queue));

    shadow::register_shadow(app);
}
