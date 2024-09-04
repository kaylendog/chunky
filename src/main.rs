use bevy::{
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};

mod channel;
mod chunk;
mod debug;
mod player;

use chunk::ChunkPlugin;
use debug::DebugPlugin;
use player::PlayerPlugin;

fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
            WireframePlugin,
            DebugPlugin,
            ChunkPlugin,
            PlayerPlugin,
        ))
        .run();
}
