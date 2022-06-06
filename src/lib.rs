use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::MaterialPipeline,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, ShaderStages,
        },
        renderer::RenderDevice,
        view::NoFrustumCulling,
    },
};

static SHADER: &str = include_str!("shader.wgsl");

const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 15204473893972682982);

const GRID_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 10583255013429636210);

pub struct InfiniteGridPlugin;

impl Plugin for InfiniteGridPlugin {
    fn build(&self, app: &mut App) {
        app.world
            .resource_mut::<Assets<Shader>>()
            .set_untracked(SHADER_HANDLE, Shader::from_wgsl(SHADER));

        app.world
            .resource_mut::<Assets<Mesh>>()
            .set_untracked(GRID_MESH_HANDLE, Mesh::from(shape::Plane { size: 1.0 }));

        app.add_plugin(MaterialPlugin::<InfiniteGridMaterial>::default());
    }
}

#[derive(TypeUuid, Copy, Clone)]
#[uuid = "dc369438-2cf9-4934-883e-59b3db6f8a9d"]
pub struct InfiniteGridMaterial {
    pub offset: Vec3,
    pub normal: Vec3,
    pub scale: f32,
    pub rot_matrix: Mat3,
    pub x_axis_color: Color,
    pub z_axis_color: Color,
}

impl Default for InfiniteGridMaterial {
    fn default() -> Self {
        Self {
            offset: Vec3::ZERO,
            normal: Vec3::Y,
            scale: 1.,
            rot_matrix: Mat3::IDENTITY,
            x_axis_color: Color::rgb(1.0, 0.2, 0.2),
            z_axis_color: Color::rgb(0.2, 0.2, 1.0),
        }
    }
}

#[derive(AsStd140)]
pub struct InfiniteGridGpuData {
    rot_matrix: Mat3,
    offset: Vec3,
    normal: Vec3,
    scale: f32,

    x_axis_color: Vec3,
    z_axis_color: Vec3,
}

impl From<InfiniteGridMaterial> for InfiniteGridGpuData {
    fn from(val: InfiniteGridMaterial) -> Self {
        Self {
            rot_matrix: val.rot_matrix,
            offset: val.offset,
            normal: val.normal,
            scale: val.scale,
            x_axis_color: Vec3::from_slice(&val.x_axis_color.as_rgba_f32()),
            z_axis_color: Vec3::from_slice(&val.z_axis_color.as_rgba_f32()),
        }
    }
}

pub struct GpuInfiniteGridMaterial {
    _buffer: Buffer,
    bind_group: BindGroup,
}

impl RenderAsset for InfiniteGridMaterial {
    type ExtractedAsset = Self;
    type PreparedAsset = GpuInfiniteGridMaterial;
    type Param = (SRes<RenderDevice>, SRes<MaterialPipeline<Self>>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        *self
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let gpu_data = InfiniteGridGpuData::from(extracted_asset);
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: gpu_data.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuInfiniteGridMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}

impl Material for InfiniteGridMaterial {
    fn vertex_shader(_: &AssetServer) -> Option<Handle<Shader>> {
        Some(SHADER_HANDLE.typed())
    }
    fn fragment_shader(_: &AssetServer) -> Option<Handle<Shader>> {
        Some(SHADER_HANDLE.typed())
    }

    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &material.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("infinite-grid-bind-group-layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        InfiniteGridGpuData::std140_size_static() as u64
                    ),
                },
                count: None,
            }],
        })
    }

    fn alpha_mode(_: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(Bundle)]
pub struct InfiniteGridBundle {
    #[bundle]
    material_mesh_bundle: MaterialMeshBundle<InfiniteGridMaterial>,
    no_frustum_culling: NoFrustumCulling,
}

impl InfiniteGridBundle {
    pub fn new(grid_material_handle: Handle<InfiniteGridMaterial>) -> Self {
        Self {
            material_mesh_bundle: MaterialMeshBundle {
                material: grid_material_handle,
                mesh: GRID_MESH_HANDLE.typed(),
                ..Default::default()
            },
            no_frustum_culling: NoFrustumCulling,
        }
    }
}
