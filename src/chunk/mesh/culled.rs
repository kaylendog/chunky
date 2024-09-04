use bevy::{
    math::{Dir3, IVec3},
    prelude::Mesh,
};
use itertools::iproduct;

use crate::chunk::{BlockPos, BlockType, CHUNK_SIZE};

use super::{triangulize, ChunkMeshBuilder, ChunkNeighbours, Quad};

/// A mesh builder that culls invisible faces.
pub struct CulledMeshBuilder {}

impl CulledMeshBuilder {
    /// Mesh the chunk in the given direction.
    fn mesh_direction<'a, F>(quads: &mut Vec<Quad>, block_at: F, dir: Dir3)
    where
        F: Fn(IVec3) -> &'a BlockType,
    {
        for (x, y, z) in iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE) {
            // check if this block is opaque
            let block = block_at(IVec3::new(x as i32, y as i32, z as i32));
            if block.is_opaque() {
                continue;
            }
            // check if previous block is opaque
            let previous: IVec3 =
                IVec3::new(x as i32, y as i32, z as i32) - Dir3::from(Dir3::X).as_ivec3();
            let block = block_at(previous.into());
            if block.is_opaque() {
                continue;
            }
            quads.push(Quad::square(IVec3::new(x as i32, y as i32, z as i32), dir));
        }
    }
}

impl ChunkMeshBuilder for CulledMeshBuilder {
    fn build(neighbours: ChunkNeighbours) -> Mesh {
        let mut quads = Vec::new();

        // east
        Self::mesh_direction(
            &mut quads,
            |pos| neighbours.block_at(IVec3::new(pos.x, pos.y, pos.z)),
            Dir3::X,
        );
        // west
        Self::mesh_direction(
            &mut quads,
            |pos| neighbours.block_at(IVec3::new(CHUNK_SIZE as i32 - pos.x, pos.y, pos.z)),
            -Dir3::X,
        );
        // south
        Self::mesh_direction(
            &mut quads,
            |pos| neighbours.block_at(IVec3::new(pos.y, pos.x, pos.z)),
            Dir3::Y,
        );
        // north
        Self::mesh_direction(
            &mut quads,
            |pos| neighbours.block_at(IVec3::new(CHUNK_SIZE as i32 - pos.y, pos.x, pos.z)),
            -Dir3::Y,
        );
        // up
        Self::mesh_direction(
            &mut quads,
            |pos| neighbours.block_at(IVec3::new(pos.z, pos.x, pos.y)),
            Dir3::Z,
        );
        // down
        Self::mesh_direction(
            &mut quads,
            |pos| neighbours.block_at(IVec3::new(CHUNK_SIZE as i32 - pos.z, pos.x, pos.y)),
            -Dir3::Z,
        );

        triangulize(quads)
    }
}
