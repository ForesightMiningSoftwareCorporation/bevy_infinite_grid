use bevy::prelude::*;
use bevy_flycam::PlayerPlugin;
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridMaterial, InfiniteGridPlugin};

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
    mut materials: ResMut<Assets<InfiniteGridMaterial>>,
) {
    commands.spawn_bundle(InfiniteGridBundle::new(
        materials.add(InfiniteGridMaterial::default()),
    ));

    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}
