use bevy::prelude::*;
use bevy_infinite_grid::{InfiniteGrid2DBundle, InfiniteGrid2DPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InfiniteGrid2DPlugin))
        .add_systems(Startup, setup_system)
        .add_systems(Update, camera_movement)
        .run();
}

fn setup_system(mut commands: Commands) {
    // Spawn the infinite 2D grid
    commands.spawn(InfiniteGrid2DBundle::default());

    // Spawn a 2D camera
    commands.spawn((
        Camera2d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0),
        CameraMovement::default(),
    ));

    // Add some 2D sprites for reference
    commands.spawn((
        Sprite {
            color: Color::srgb(0.8, 0.2, 0.2),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        Transform::from_xyz(100.0, 100.0, 1.0),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.8, 0.2),
            custom_size: Some(Vec2::new(30.0, 30.0)),
            ..default()
        },
        Transform::from_xyz(-150.0, 50.0, 1.0),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.2, 0.8),
            custom_size: Some(Vec2::new(40.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -200.0, 1.0),
    ));
}

#[derive(Component)]
struct CameraMovement {
    speed: f32,
}

impl Default for CameraMovement {
    fn default() -> Self {
        Self { speed: 200.0 }
    }
}

fn camera_movement(
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<(&mut Transform, &CameraMovement), With<Camera2d>>,
) {
    let Ok((mut transform, movement)) = camera_query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    
    let mut direction = Vec2::ZERO;
    
    if key_input.pressed(KeyCode::KeyW) || key_input.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if key_input.pressed(KeyCode::KeyS) || key_input.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if key_input.pressed(KeyCode::KeyA) || key_input.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if key_input.pressed(KeyCode::KeyD) || key_input.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    
    if direction != Vec2::ZERO {
        direction = direction.normalize();
        let movement_delta = direction * movement.speed * dt;
        transform.translation.x += movement_delta.x;
        transform.translation.y += movement_delta.y;
    }
}