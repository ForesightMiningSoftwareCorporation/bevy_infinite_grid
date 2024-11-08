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
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 4.37, 14.77),
            ..default()
        },
        RenderLayers::layer(1),
    ));

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_translation(Vec3::X * 15. + Vec3::Y * 20.)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let mat = standard_materials.add(StandardMaterial::default());

    // cube
    commands.spawn(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Cuboid::from_size(Vec3::ONE).mesh()),
        transform: Transform {
            translation: Vec3::new(3., 4., 0.),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            scale: Vec3::splat(1.5),
        },
        ..default()
    });

    commands.spawn(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Cuboid::from_size(Vec3::ONE).mesh()),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..default()
    });
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
