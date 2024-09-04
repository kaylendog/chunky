mod culled;
mod stupid;

use bevy::{
    math::{Dir3, IVec3, Vec3},
    prelude::Mesh,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};
use culled::CulledMeshBuilder;
use itertools::{iproduct, Itertools};

use super::{BlockPos, BlockType, Chunk, CHUNK_SIZE};

/// Chunk size minus one.
const CHUNK_SIZE_MINUS_ONE: u8 = CHUNK_SIZE - 1;

/// Chunk size plus one.
const CHUNK_SIZE_PLUS_ONE: i32 = CHUNK_SIZE as i32 + 1;

/// Square of the chunk size.
const CHUNK_SIZE_2: usize = CHUNK_SIZE as usize * CHUNK_SIZE as usize;

/// Padded chunk size for the mesh.
const CHUNK_SIZE_PADDED: usize = CHUNK_SIZE as usize + 2;

/// Square of the padded chunk size.
const CHUNK_SIZE_PADDED_2: usize = CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED;

/// A mesh builder for chunks.
pub trait ChunkMeshBuilder {
    /// Builds a mesh for a chunk.
    fn build(data: ChunkNeighbours) -> Mesh;
}

/// A struct that stores neighbours of a chunk.
pub struct ChunkNeighbours<'a> {
    pub chunk: &'a Chunk,
    pub north: &'a Chunk,
    pub east: &'a Chunk,
    pub south: &'a Chunk,
    pub west: &'a Chunk,
    pub up: &'a Chunk,
    pub down: &'a Chunk,
}

impl<'a> ChunkNeighbours<'a> {
    /// Returns the block at the given position, with neighbours taken into account.
    pub fn block_at(&self, IVec3 { x, y, z }: IVec3) -> &BlockType {
        match (x, y, z) {
            (-1, _, _) => self.west.block_at((CHUNK_SIZE_MINUS_ONE, y as u8, z as u8)),
            (CHUNK_SIZE_PLUS_ONE, _, _) => self.east.block_at((0, y as u8, z as u8)),
            (_, -1, _) => self.down.block_at((x as u8, CHUNK_SIZE_MINUS_ONE, z as u8)),
            (_, CHUNK_SIZE_PLUS_ONE, _) => self.up.block_at((x as u8, 0, z as u8)),
            (_, _, -1) => self
                .south
                .block_at((x as u8, y as u8, CHUNK_SIZE_MINUS_ONE)),
            (_, _, CHUNK_SIZE_PLUS_ONE) => self.north.block_at((x as u8, y as u8, 0)),
            _ => self.chunk.block_at((x as u8, y as u8, z as u8)),
        }
    }

    /// Return an iterator over all blocks in the chunk, ordered by their position.
    pub fn blocks(&self) -> impl Iterator<Item = (IVec3, BlockType)> + '_ {
        iproduct!(
            -1..CHUNK_SIZE_PLUS_ONE,
            -1..CHUNK_SIZE_PLUS_ONE,
            -1..CHUNK_SIZE_PLUS_ONE
        )
        .map(move |pos| {
            let block = self.block_at(pos.into());
            (pos.into(), *block)
        })
    }
}

/// A struct that stores the vertices and indices of a mesh.
pub struct Quad {
    /// The vertices of the quad.
    pub vertices: [IVec3; 4],
}

pub enum Face {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl From<Face> for Dir3 {
    fn from(face: Face) -> Self {
        match face {
            Face::North => Dir3::Z,
            Face::East => Dir3::X,
            Face::South => Dir3::NEG_Z,
            Face::West => Dir3::NEG_X,
            Face::Up => Dir3::Y,
            Face::Down => Dir3::NEG_Y,
        }
    }
}

impl Quad {
    /// Create a list of quads for the given block position.
    pub fn faces(pos: BlockPos) -> [Quad; 6] {
        let BlockPos { x, y, z } = pos;
        let x = x as i32;
        let y = y as i32;
        let z = z as i32;
        [
            // north
            Quad::square(IVec3::new(x, y, z), Dir3::NEG_Z),
            // east
            Quad::square(IVec3::new(x + 1, y, z), Dir3::X),
            // south
            Quad::square(IVec3::new(x + 1, y, z + 1), Dir3::Z),
            // west
            Quad::square(IVec3::new(x, y, z + 1), Dir3::NEG_X),
            // up
            Quad::square(IVec3::new(x, y + 1, z + 1), Dir3::Y),
            // down
            Quad::square(IVec3::new(x, y, z), Dir3::NEG_Y),
        ]
    }

    pub fn square(pos: IVec3, direction: Dir3) -> Quad {
        Quad::new(pos, direction, 1, 1)
    }

    /// Creates a new quad from a rectangle. The quad's normal will be in the right-hand normal direction.
    pub fn new(pos: IVec3, direction: Dir3, width: u32, height: u32) -> Quad {
        let normal = direction.as_vec3();

        // handle up and down directions separately - cross
        let up = if direction == Dir3::Y || direction == Dir3::NEG_Y {
            Vec3::X
        } else {
            Vec3::Y
        };
        let right = normal.cross(up);

        // counter-clockwise when looking at the normal
        let a = pos.as_vec3();
        let b = pos.as_vec3() + up * height as f32;
        let c = pos.as_vec3() + up * height as f32 + right * width as f32;
        let d = pos.as_vec3() + right * width as f32;

        Quad {
            vertices: [a.as_ivec3(), b.as_ivec3(), c.as_ivec3(), d.as_ivec3()],
        }
    }

    /// Calculates the normal of the quad.
    #[inline]
    pub fn normal(&self) -> Vec3 {
        let a = self.vertices[0];
        let b = self.vertices[1];
        let c = self.vertices[2];
        let d = self.vertices[3];

        let ab = b - a;
        let ac = c - a;
        let ad = d - a;

        (ab.cross(ac) + ac.cross(ad)).as_vec3().normalize()
    }
}

/// Triangulizes a list of quads.
pub fn triangulize(quads: Vec<Quad>) -> Mesh {
    // mesh properties
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();

    for quad in quads {
        // append vertices
        let start = vertices.len() as u32;
        for vertex in &quad.vertices {
            vertices.push(*vertex);
        }
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        // push normal for each vertex
        let normal = quad.normal();
        for _ in 0..4 {
            normals.push(normal);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vertices.into_iter().map(|v| v.as_vec3()).collect_vec(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

pub fn build(data: ChunkNeighbours) -> Mesh {
    CulledMeshBuilder::build(data)
}
