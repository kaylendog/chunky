use bevy::{
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
    window::CursorGrabMode,
};
use itertools::iproduct;

use crate::chunk::{ChunkCommand, ChunkPos, Chunks};

/// A marker component for player entities.
#[derive(Component, Default)]
struct Player;

/// A player entity.
#[derive(Bundle, Default)]
struct PlayerBundle {
    player: Player,
    transform: Transform,
    global_transform: GlobalTransform,
}

/// A plugin for handling player input and processing.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player).add_systems(
            Update,
            (
                // movement
                lock_cursor,
                move_player,
                rotate_camera,
                // chunk
                load_chunks_near_player,
            ),
        );
    }
}

/// Spawn the player entity.
fn spawn_player(mut commands: Commands) {
    commands
        .spawn(PlayerBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(Camera3dBundle::default());
            parent.spawn(PointLightBundle {
                point_light: PointLight {
                    intensity: 100_000_000.0,
                    range: 1024.0,
                    ..default()
                },
                ..default()
            });
        });
}

fn move_player(
    _: Commands,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut transform = query.single_mut();
    let forward = transform.forward();
    let right = transform.right();

    // lateral movement
    let mut direction = Vec3::ZERO;
    if input.pressed(KeyCode::KeyW) {
        direction += *forward;
    }
    if input.pressed(KeyCode::KeyS) {
        direction -= *forward;
    }
    if input.pressed(KeyCode::KeyA) {
        direction -= *right;
    }
    if input.pressed(KeyCode::KeyD) {
        direction += *right;
    }

    // adjust speed based on modifier keys
    let speed_factor = match input.pressed(KeyCode::ControlLeft) {
        true => 10.0,
        false => 5.0,
    };

    transform.translation += direction.normalize_or_zero() * time.delta_seconds() * speed_factor;

    // vertical movement
    let mut direction = 0.0;

    if input.pressed(KeyCode::Space) {
        direction += 1.0;
    }
    if input.pressed(KeyCode::ShiftLeft) {
        direction -= 1.0;
    }

    transform.translation.y += direction as f32 * time.delta_seconds() * 5.0;
}

fn rotate_camera(
    _: Commands,
    mut mouse_events: EventReader<MouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    let mut player_transform = player_query.single_mut();
    let mut camera_transform = camera_query.single_mut();

    for event in mouse_events.read() {
        player_transform.rotate(Quat::from_rotation_y(-event.delta.x * 0.001));
        camera_transform.rotate(Quat::from_rotation_x(-event.delta.y * 0.001));
    }
}

fn lock_cursor(
    mut windows: Query<&mut Window>,
    mouse_events: EventReader<MouseButtonInput>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let mut window = windows.single_mut();
    // lock cursor when mouse button is pressed (focus gained)
    if !mouse_events.is_empty() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
    // unlock cursor when escape key is pressed
    if input.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}

fn load_chunks_near_player(
    query: Query<&Transform, With<Player>>,
    chunks: Res<Chunks>,
    mut events: EventWriter<ChunkCommand>,
) {
    let player_chunk = ChunkPos::from_world(query.single().translation);

    // unload chunks in 10x10 radius
    events.send_batch(
        iproduct!(0..2, 0..2, 0..2)
            .filter_map(|diff| {
                let pos = player_chunk + diff.into();
                match chunks.is_unloaded(pos) {
                    true => Some(pos),
                    false => None,
                }
            })
            .map(|pos| ChunkCommand::Load(pos)),
    );

    // unload far chunks
    events.send_batch(chunks.iter().filter_map(|chunk| {
        let distance = (chunk.position - player_chunk).max().abs();
        match distance {
            0..=10 => None,
            _ => Some(ChunkCommand::Unload(chunk.position)),
        }
    }));
}
