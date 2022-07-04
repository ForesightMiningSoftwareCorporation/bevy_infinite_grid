use bevy::prelude::*;
use bevy_flycam::PlayerPlugin;
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InfiniteGridPlugin)
        .add_plugin(PlayerPlugin)
        .add_startup_system(setup_system)
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
}
