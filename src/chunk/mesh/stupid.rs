use bevy::prelude::Mesh;
use itertools::Itertools;

use super::{triangulize, ChunkMeshBuilder, ChunkNeighbours, Quad};

pub struct StupidMeshBuilder;

impl ChunkMeshBuilder for StupidMeshBuilder {
    fn build(neighbours: ChunkNeighbours) -> Mesh {
        // just collect all faces and triangulize them
        let quads = neighbours
            .chunk
            .blocks()
            .flat_map(|(pos, _)| Quad::faces(pos).into_iter())
            .collect_vec();
        triangulize(quads)
    }
}
