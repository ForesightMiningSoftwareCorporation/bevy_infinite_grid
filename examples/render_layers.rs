use bevy::{prelude::*, render::view::RenderLayers};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InfiniteGridPlugin))
        .add_systems(Startup, setup_system)
        .add_systems(Update, toggle_layers)
        .run();
}

fn setup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((InfiniteGridBundle::default(), RenderLayers::layer(1)));

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Transform::from_xyz(0.0, 4.37, 14.77),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        DirectionalLight { ..default() },
        Transform::from_translation(Vec3::X * 15. + Vec3::Y * 20.).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(standard_materials.add(StandardMaterial::default())),
        Transform::from_xyz(3.0, 4.0, 0.0)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()))
            .with_scale(Vec3::splat(1.5)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(standard_materials.add(StandardMaterial::default())),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));
}

fn toggle_layers(mut q: Query<&mut RenderLayers, With<Camera>>, input: Res<ButtonInput<KeyCode>>) {
    for mut render_layers in &mut q {
        if input.just_pressed(KeyCode::KeyT) {
            if render_layers.intersects(&RenderLayers::layer(1)) {
                *render_layers = RenderLayers::layer(0);
            } else {
                *render_layers = RenderLayers::layer(1);
            }
        }
    }
}
