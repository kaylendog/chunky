use bevy::{pbr::wireframe::Wireframe, prelude::*};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_debug_cube);
        // .add_systems(Update, draw_debug_gizmos);
    }
}

pub fn spawn_debug_cube(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            transform: Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            ..Default::default()
        },
        Wireframe,
    ));
}

pub fn draw_debug_gizmos(mut gizmos: Gizmos, query: Query<&Transform, With<Camera>>) {
    let transform = query.single();
    let origin = transform.forward() * 10.0 + transform.translation;
    gizmos.arrow(origin, origin + Vec3::X, Color::srgb(1.0, 0.0, 0.0));
    gizmos.arrow(origin, origin + Vec3::Y, Color::srgb(0.0, 1.0, 0.0));
    gizmos.arrow(origin, origin + Vec3::Z, Color::srgb(0.0, 0.0, 1.0));
}
