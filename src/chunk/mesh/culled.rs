use bevy::{
    math::{Dir3, IVec3},
    prelude::Mesh,
};
use itertools::iproduct;

use crate::chunk::{BlockPos, BlockType, CHUNK_SIZE};

use super::{triangulize, ChunkMeshBuilder, ChunkNeighbours, Quad};

/// A mesh builder that culls invisible faces.
pub struct CulledMeshBuilder {}

impl CulledMeshBuilder {}

impl ChunkMeshBuilder for CulledMeshBuilder {
    fn build(neighbours: ChunkNeighbours) -> Mesh {
        let mut quads = Vec::with_capacity(CHUNK_SIZE as usize * CHUNK_SIZE as usize * 6);
        for (pos, block) in neighbours.chunk.blocks() {
            if !block.is_opaque() {
                continue;
            }
            for face in Quad::faces(pos) {
                let dir = face.normal();
                let neighbour_block = neighbours.block_at(IVec3::from(pos) + dir.as_ivec3());
                if !neighbour_block.is_opaque() {
                    quads.push(face);
                }
            }
        }
        triangulize(quads)
    }
}
