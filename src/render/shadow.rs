use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{DrawMesh, MeshPipeline, NotShadowCaster, SetMeshBindGroup, ShadowPipelineKey},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::CameraProjection,
        mesh::MeshVertexBufferLayout,
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraph},
        render_phase::{
            AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions,
            EntityPhaseItem, EntityRenderCommand, PhaseItem, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BufferBindingType, BufferSize, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            Extent3d, FilterMode, FragmentState, FrontFace, LoadOp, MultisampleState, Operations,
            PipelineCache, PolygonMode, PrimitiveState, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerDescriptor,
            ShaderStages, ShaderType, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, TextureView, VertexState,
        },
        renderer::RenderDevice,
        texture::TextureCache,
        view::{
            ExtractedView, ExtractedWindows, ViewUniform, ViewUniformOffset, ViewUniforms,
            VisibleEntities,
        },
        RenderApp, RenderStage,
    },
    utils::FloatOrd,
    window::WindowId,
};

use crate::{GridFrustumIntersect, InfiniteGridSettings};

use super::{
    ExtractedInfiniteGrid, GridShadowUniformOffset, GridShadowUniforms, InfiniteGridPipeline,
};

static SHADOW_RENDER: &str = include_str!("shadow_render.wgsl");

const SHADOW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 10461510954165139918);

pub struct GridShadow {
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for GridShadow {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        unimplemented!("grid shadows don't need sorting")
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for GridShadow {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for GridShadow {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

#[derive(Resource)]
pub struct GridShadowPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub skinned_mesh_layout: BindGroupLayout,
    pub sampler: Sampler,
}

impl FromWorld for GridShadowPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(ViewUniform::min_size().into()),
                    },
                    count: None,
                },
            ],
            label: Some("grid_shadow_view_layout"),
        });

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();
        let skinned_mesh_layout = mesh_pipeline.skinned_mesh_layout.clone();

        GridShadowPipeline {
            view_layout,
            mesh_layout: mesh_pipeline.mesh_layout.clone(),
            skinned_mesh_layout,
            sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: None,
                ..Default::default()
            }),
        }
    }
}

impl SpecializedMeshPipeline for GridShadowPipeline {
    type Key = ShadowPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut vertex_attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];

        let mut bind_group_layout = vec![self.view_layout.clone()];
        let mut shader_defs = Vec::new();

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            shader_defs.push(String::from("SKINNED"));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
            bind_group_layout.push(self.skinned_mesh_layout.clone());
        } else {
            bind_group_layout.push(self.mesh_layout.clone());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SHADOW_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SHADOW_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: ColorWrites::RED,
                })],
            }),
            layout: Some(bind_group_layout),
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            label: Some("grid_shadow_pipeline".into()),
        })
    }
}

#[derive(Resource, Default)]
struct GridShadowMeta {
    view_bind_group: Option<BindGroup>,
}

type DrawGridShadowMesh = (
    SetItemPipeline,
    SetGridShadowViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

struct SetGridShadowViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetGridShadowViewBindGroup<I> {
    type Param = (SRes<GridShadowMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (meta, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform_offset = query.get(view).unwrap();
        pass.set_bind_group(
            I,
            meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform_offset.offset],
        );

        RenderCommandResult::Success
    }
}

#[derive(Component)]
struct GridShadowView {
    texture_view: TextureView,
}

fn prepare_grid_shadow_views(
    mut commands: Commands,
    grids: Query<(Entity, &ExtractedInfiniteGrid, &GridFrustumIntersect)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    windows: Res<ExtractedWindows>,
    settings: Res<RenderSettings>,
) {
    let primary_window = if let Some(w) = windows.get(&WindowId::primary()) {
        w
    } else {
        return;
    };
    let width = primary_window.physical_width;
    let height = primary_window.physical_height;
    let comp = width < height;
    let [min, max] = if comp {
        [width, height]
    } else {
        [height, width]
    };
    let ratio = min as f32 / max as f32;
    let tmax = settings.max_texture_size;
    let tmin = (tmax as f32 * ratio) as u32;
    let [width, height] = if comp { [tmin, tmax] } else { [tmax, tmin] };
    for (entity, grid, frustum_intersect) in grids.iter() {
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("grid_shadow_texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );

        let projection = OrthographicProjection {
            bottom: frustum_intersect.height / -2.,
            top: frustum_intersect.height / 2.,
            left: frustum_intersect.width / -2.,
            right: frustum_intersect.width / 2.,
            ..Default::default()
        };

        commands.entity(entity).insert((
            ExtractedView {
                projection: projection.get_projection_matrix(),
                transform: Transform::from_translation(
                    frustum_intersect.center + grid.transform.up() * 500.,
                )
                .looking_at(frustum_intersect.center, frustum_intersect.up_dir)
                .into(),
                hdr: false,
                viewport: UVec4::new(0, 0, width, height),
            },
            GridShadowView {
                texture_view: texture.default_view.clone(),
            },
        ));
    }
}

fn queue_grid_shadow_view_bind_group(
    render_device: Res<RenderDevice>,
    shadow_pipeline: Res<GridShadowPipeline>,
    mut meta: ResMut<GridShadowMeta>,
    view_uniforms: Res<ViewUniforms>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("grid_shadow_view_bind_group"),
            layout: &shadow_pipeline.view_layout,
        }));
    }
}

#[derive(Component)]
pub struct GridShadowBindGroup {
    bind_group: BindGroup,
}

fn queue_grid_shadow_bind_groups(
    mut commands: Commands,
    grids: Query<(Entity, &GridShadowView)>,
    uniforms: Res<GridShadowUniforms>,
    infinite_grid_pipeline: Res<InfiniteGridPipeline>,
    grid_shadow_pipeline: Res<GridShadowPipeline>,
    render_device: Res<RenderDevice>,
) {
    if let Some(uniform_binding) = uniforms.uniforms.binding() {
        for (entity, shadow_view) in grids.iter() {
            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("grid-shadow-bind-group"),
                layout: &infinite_grid_pipeline.grid_shadows_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: uniform_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&shadow_view.texture_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&grid_shadow_pipeline.sampler),
                    },
                ],
            });
            commands
                .entity(entity)
                .insert(GridShadowBindGroup { bind_group });
        }
    }
}

fn queue_grid_shadows(
    mut grids: Query<(&mut RenderPhase<GridShadow>, &VisibleEntities)>,
    casting_meshes: Query<&Handle<Mesh>, Without<NotShadowCaster>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GridShadowPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    shadow_pipeline: Res<GridShadowPipeline>,
    shadow_draw_functions: Res<DrawFunctions<GridShadow>>,
) {
    let draw_shadow_mesh = shadow_draw_functions
        .read()
        .get_id::<DrawGridShadowMesh>()
        .unwrap();
    for (mut phase, entities) in grids.iter_mut() {
        for &entity in &entities.entities {
            if let Ok(mesh_handle) = casting_meshes.get(entity) {
                if let Some(mesh) = render_meshes.get(mesh_handle) {
                    let key = ShadowPipelineKey::from_primitive_topology(mesh.primitive_topology);
                    let pipeline_id = pipelines.specialize(
                        &mut pipeline_cache,
                        &shadow_pipeline,
                        key,
                        &mesh.layout,
                    );

                    let pipeline_id = match pipeline_id {
                        Ok(id) => id,
                        Err(err) => {
                            error!("{}", err);
                            continue;
                        }
                    };

                    phase.add(GridShadow {
                        draw_function: draw_shadow_mesh,
                        pipeline: pipeline_id,
                        entity,
                    });
                }
            }
        }
    }
}

pub struct SetGridShadowBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetGridShadowBindGroup<I> {
    type Param = SQuery<(Read<GridShadowBindGroup>, Read<GridShadowUniformOffset>)>;

    fn render<'w>(
        _view: Entity,
        item: Entity,
        query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Ok((bg, offset)) = query.get_inner(item) {
            pass.set_bind_group(I, &bg.bind_group, &[offset.offset]);
        }
        RenderCommandResult::Success
    }
}

struct GridShadowPassNode {
    grids: Vec<Entity>,
    grid_entity_query: QueryState<Entity, With<GridShadowView>>,
    grid_element_query: QueryState<(Read<GridShadowView>, Read<RenderPhase<GridShadow>>)>,
}

impl GridShadowPassNode {
    const NAME: &'static str = "grid_shadow_pass";

    fn new(world: &mut World) -> Self {
        Self {
            grids: Vec::new(),
            grid_entity_query: world.query_filtered(),
            grid_element_query: world.query(),
        }
    }
}

impl Node for GridShadowPassNode {
    fn update(&mut self, world: &mut World) {
        self.grids.clear();
        self.grids.extend(self.grid_entity_query.iter(world));
        self.grid_element_query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        for &entity in &self.grids {
            let (shadow_view, render_phase) =
                self.grid_element_query.get_manual(world, entity).unwrap();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("grid_shadow_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &shadow_view.texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK.into()),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            };

            let draw_functions = world.resource::<DrawFunctions<GridShadow>>();
            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in &render_phase.items {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, entity, item);
            }
        }

        Ok(())
    }
}

#[derive(Reflect, Resource, Clone)]
pub struct RenderSettings {
    pub max_texture_size: u32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            max_texture_size: 16384,
        }
    }
}

pub fn register_shadow(app: &mut App) {
    app.world
        .resource_mut::<Assets<Shader>>()
        .set_untracked(SHADOW_SHADER_HANDLE, Shader::from_wgsl(SHADOW_RENDER));

    let render_settings = app
        .world
        .resource::<InfiniteGridSettings>()
        .render_settings
        .clone();

    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app
        .init_resource::<GridShadowMeta>()
        .init_resource::<GridShadowPipeline>()
        .init_resource::<DrawFunctions<GridShadow>>()
        .init_resource::<SpecializedMeshPipelines<GridShadowPipeline>>()
        .insert_resource(render_settings)
        .add_render_command::<GridShadow, DrawGridShadowMesh>()
        .add_system_to_stage(
            RenderStage::Prepare,
            // Register as exclusive system because ordering against `bevy_render::view::prepare_view_uniforms` isn't possible otherwise.
            prepare_grid_shadow_views.at_start(),
        )
        .add_system_to_stage(RenderStage::Queue, queue_grid_shadows)
        .add_system_to_stage(RenderStage::Queue, queue_grid_shadow_bind_groups)
        .add_system_to_stage(RenderStage::Queue, queue_grid_shadow_view_bind_group);

    let grid_shadow_pass_node = GridShadowPassNode::new(&mut render_app.world);
    let mut graph = render_app.world.resource_mut::<RenderGraph>();
    let draw_3d_graph = graph
        .get_sub_graph_mut(bevy::core_pipeline::core_3d::graph::NAME)
        .unwrap();
    draw_3d_graph.add_node(GridShadowPassNode::NAME, grid_shadow_pass_node);
    draw_3d_graph
        .add_node_edge(
            GridShadowPassNode::NAME,
            bevy::core_pipeline::core_3d::graph::node::MAIN_PASS,
        )
        .unwrap();
}
