use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    render::{camera::Camera3d, mesh::VertexAttributeValues, view::NoFrustumCulling},
};
use bevy_flycam::PlayerPlugin;
use bevy_infinite_grid::{
    GridFrustumIntersect, InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InfiniteGridPlugin)
        .add_plugin(PlayerPlugin)
        .add_startup_system(setup_system)
        .add_system_to_stage(CoreStage::PostUpdate, adjust_plane_system)
        .add_system(distant_point_system)
        .run();
}

fn setup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(InfiniteGridBundle::default());

    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_translation(Vec3::X * 15. + Vec3::Y * 20.)
            .looking_at(Vec3::ZERO, Vec3::Y),
        directional_light: DirectionalLight {
            ..Default::default()
        },
        ..Default::default()
    });

    let mat = standard_materials.add(StandardMaterial::default());

    // cube
    commands.spawn_bundle(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform {
            translation: Vec3::new(3., 4., 0.),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            scale: Vec3::splat(1.5),
        },
        ..default()
    });

    commands.spawn_bundle(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..default()
    });

    commands
        .spawn_bundle(PbrBundle {
            material: mat.clone(),
            mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
            transform: Transform::from_xyz(0.0, 0.0, -100.0),
            ..default()
        })
        .insert(DistantPointMarker);

    commands
        .spawn_bundle(PbrBundle {
            material: standard_materials.add(StandardMaterial {
                cull_mode: None,
                base_color: Color::BLACK,
                ..Default::default()
            }),
            mesh: meshes.add(Mesh::from(shape::Plane { size: 1.0 })),
            ..Default::default()
        })
        .insert(PlaneMarker)
        .insert(NoFrustumCulling);
}

#[derive(Component)]
struct PlaneMarker;

#[derive(Component)]
struct DistantPointMarker;

fn calculate_distant_from(cam: &GlobalTransform, grid: &GlobalTransform) -> Vec3 {
    let cam_pos = cam.translation;
    let cam_dir = cam.local_z();

    let inverse_rot = grid.rotation.inverse();

    let gs_cam_pos = inverse_rot * (cam_pos - grid.translation);
    let gs_cam_dir = inverse_rot * cam_dir;

    let pos_in_grid_space = (gs_cam_pos.xz() - gs_cam_dir.xz().normalize() * 200. * grid.scale.x)
        .extend(0.)
        .xzy();

    grid.translation + pos_in_grid_space
}

fn distant_point_system(
    grid: Query<&GlobalTransform, With<InfiniteGrid>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut distant: Query<&mut Transform, With<DistantPointMarker>>,
) {
    let cam_pos = camera.single();
    distant.single_mut().translation = calculate_distant_from(cam_pos, grid.single());
}

fn adjust_plane_system(
    grid: Query<&GridFrustumIntersect, With<InfiniteGrid>>,
    debug_plane: Query<&Handle<Mesh>, With<PlaneMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    input: Res<Input<KeyCode>>,
    mut toggle: Local<bool>,
) {
    if input.just_pressed(KeyCode::H) {
        *toggle = !*toggle;
    }
    if *toggle {
        return;
    }

    let mesh = meshes.get_mut(debug_plane.single()).unwrap();
    let positions = match mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(positions)) => positions,
        _ => unreachable!(),
    };

    positions.clear();
    positions.extend(grid.single().points.map(<[f32; 3]>::from));
}
