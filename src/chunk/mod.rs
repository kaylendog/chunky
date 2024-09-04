mod mesh;

use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::Debug,
    ops::{Add, Sub},
};

use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use itertools::iproduct;
use mesh::ChunkNeighbours;
use noise::NoiseFn;

/// The size of a chunk along one axis, measured in blocks.
pub const CHUNK_SIZE: u8 = 32;

/// A position of a chunk in the world in chunk coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl From<(i64, i64, i64)> for ChunkPos {
    fn from((x, y, z): (i64, i64, i64)) -> Self {
        Self::new(x, y, z)
    }
}

impl Add for ChunkPos {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for ChunkPos {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl ChunkPos {
    const NORTH: Self = Self { x: 0, y: 0, z: -1 };
    const EAST: Self = Self { x: 1, y: 0, z: 0 };
    const SOUTH: Self = Self { x: 0, y: 0, z: 1 };
    const WEST: Self = Self { x: -1, y: 0, z: 0 };
    const UP: Self = Self { x: 0, y: 1, z: 0 };
    const DOWN: Self = Self { x: 0, y: -1, z: 0 };

    /// Create a new chunk position.
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    /// Create a new chunk position from a world position.
    pub fn from_world(pos: Vec3) -> Self {
        Self {
            x: pos.x as i64 / CHUNK_SIZE as i64,
            y: pos.y as i64 / CHUNK_SIZE as i64,
            z: pos.z as i64 / CHUNK_SIZE as i64,
        }
    }

    pub fn to_world(&self) -> Vec3 {
        Vec3::new(
            self.x as f32 * CHUNK_SIZE as f32,
            self.y as f32 * CHUNK_SIZE as f32,
            self.z as f32 * CHUNK_SIZE as f32,
        )
    }

    /// Return an iterator over the neighboring chunk positions.
    pub fn neighbors(&self, radius: i64) -> impl Iterator<Item = ChunkPos> + '_ {
        iproduct!(-radius..=radius, -radius..=radius, -radius..=radius)
            .filter(|&(dx, dy, dz)| dx != 0 || dy != 0 || dz != 0)
            .map(move |(dx, dy, dz)| ChunkPos::new(self.x + dx, self.y + dy, self.z + dz))
    }

    /// Return the largest component
    pub fn max(&self) -> i64 {
        self.x.max(self.y.max(self.z))
    }
}

/// A position of a block within a chunk in block coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl PartialOrd for BlockPos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BlockPos {
    fn cmp(&self, other: &Self) -> Ordering {
        // enforce zxy order
        self.z
            .cmp(&other.z)
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
    }
}

impl Add for BlockPos {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for BlockPos {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl BlockPos {
    /// Create a new block position.
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }

    /// Return an iterator over all block positions in a chunk.
    pub fn all() -> impl Iterator<Item = BlockPos> {
        iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE).map(|pos| pos.into())
    }

    pub fn world_pos(&self, chunk_pos: ChunkPos) -> Vec3 {
        Vec3::new(
            (chunk_pos.x * CHUNK_SIZE as i64 + self.x as i64) as f32,
            (chunk_pos.y * CHUNK_SIZE as i64 + self.y as i64) as f32,
            (chunk_pos.z * CHUNK_SIZE as i64 + self.z as i64) as f32,
        )
    }
}

impl From<(u8, u8, u8)> for BlockPos {
    fn from((x, y, z): (u8, u8, u8)) -> Self {
        Self::new(x, y, z)
    }
}

impl Into<(u8, u8, u8)> for BlockPos {
    fn into(self) -> (u8, u8, u8) {
        (self.x, self.y, self.z)
    }
}

impl From<BlockPos> for IVec3 {
    fn from(pos: BlockPos) -> IVec3 {
        IVec3::new(pos.x as i32, pos.y as i32, pos.z as i32)
    }
}

/// The data of a chunk.
pub struct Chunk {
    /// The position of the chunk in the world.
    pub position: ChunkPos,
    /// The block data of the chunk.
    data: BTreeMap<BlockPos, BlockType>,
}

impl Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("position", &self.position)
            .field("blocks", &self.data.len())
            .finish()
    }
}

impl Chunk {
    /// Create an empty chunk.
    pub fn empty(position: ChunkPos) -> Self {
        Self {
            position,
            data: BTreeMap::new(),
        }
    }

    /// Create a chunk filled with a block.
    pub fn filled(mut self, block: BlockType) -> Self {
        self.fill(block);
        self
    }

    /// Get the block at the given position.
    pub fn block_at<I: Into<BlockPos>>(&self, pos: I) -> &BlockType {
        &self.data.get(&pos.into()).unwrap_or(&BlockType::Empty)
    }

    /// Return an iterator over all blocks in the chunk, ordered by their position.
    pub fn blocks(&self) -> impl Iterator<Item = (BlockPos, BlockType)> + '_ {
        BlockPos::all().filter_map(move |pos| self.data.get(&pos).map(|&block| (pos, block)))
    }

    /// Generate the chunk.
    fn generate_mut(&mut self, noise: &noise::OpenSimplex) {
        for (x, y, z) in iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE) {
            let nx = (self.position.x * CHUNK_SIZE as i64 + x as i64) as f64;
            let ny = (self.position.y * CHUNK_SIZE as i64 + y as i64) as f64;
            let nz = (self.position.z * CHUNK_SIZE as i64 + z as i64) as f64;
            let value = noise.get([nx / 10.0, ny / 10.0, nz / 10.0]);
            if value > 0.0 {
                self.set_block((x, y, z), BlockType::Stone);
            }
        }
    }

    /// Set the block at the given position.
    fn set_block<Pos: Into<BlockPos>>(&mut self, pos: Pos, block: BlockType) {
        self.data.insert(pos.into(), block);
    }

    /// Fill the chunk with a block.
    fn fill(&mut self, block: BlockType) {
        for pos in BlockPos::all() {
            self.set_block(pos, block);
        }
    }
}

/// The type of a block in the world.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    #[default]
    Empty,
    Stone,
}

impl BlockType {
    /// Check if this block is opaque.
    pub fn is_opaque(&self) -> bool {
        match self {
            Self::Stone => true,
            _ => false,
        }
    }
}

/// A collection of chunks.
#[derive(Default, Resource)]
pub struct Chunks {
    /// A set of chunks that are busy, i.e. undergoing loading, unloading, or mesh building.
    busy: HashSet<ChunkPos>,
    /// A map of chunk positions to chunks.
    chunks: HashMap<ChunkPos, Chunk>,
}

impl Chunks {
    /// Get the chunk at the given position.
    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.get(pos).is_some()
    }

    /// Check if the chunk at the given position is busy.
    pub fn is_busy(&self, pos: ChunkPos) -> bool {
        self.busy.contains(&pos)
    }

    /// Check if the chunk at the given position is unloaded.
    pub fn is_unloaded(&self, pos: ChunkPos) -> bool {
        !self.is_loaded(pos) && !self.is_busy(pos)
    }

    /// Get the chunk at the given position.
    pub fn get(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    /// Return an iterator over loaded chunks.
    pub fn iter(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.values()
    }
}

/// An enumeration of events related to chunks.
#[derive(Event)]
pub enum ChunkCommand {
    /// Load a chunk at the given position.
    Load(ChunkPos),
    /// Unload a chunk at the given position.
    Unload(ChunkPos),
    /// Modify a block at the given position.
    ModifyBlock(ChunkPos, BlockPos, BlockType),
}

#[derive(Event)]
pub enum ChunkEvent {
    /// The chunk was successfully loaded.
    LoadComplete(Chunk, Mesh),
    /// The chunk was successfully unloaded.
    UnloadComplete(ChunkPos),
}

/// A component for storing a running chunk task.
#[derive(Component)]
struct ChunkTask(Task<anyhow::Result<ChunkEvent>>);

/// Plugin for handling chunk events.
pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChunkCommand>()
            .init_resource::<Chunks>()
            .add_systems(PreUpdate, poll_chunk_events)
            .add_systems(PostUpdate, process_chunk_commands);
    }
}

/// System that processes
fn process_chunk_commands(
    mut commands: Commands,
    mut chunk_commands: EventReader<ChunkCommand>,
    mut chunks: ResMut<Chunks>,
) {
    let pool = AsyncComputeTaskPool::get();
    if chunk_commands.len() != 0 {
        info!("Processing {} chunk commands", chunk_commands.len());
    }
    for chunk_command in chunk_commands.read() {
        let task = match chunk_command {
            ChunkCommand::Load(pos) => {
                chunks.busy.insert(*pos);
                pool.spawn(load_chunk(*pos))
            }
            ChunkCommand::Unload(pos) => {
                chunks.busy.insert(*pos);
                pool.spawn(unload_chunk(*pos))
            }
            ChunkCommand::ModifyBlock(pos, block_pos, block) => {
                pool.spawn(modify_block(*pos, *block_pos, *block))
            }
        };
        commands.spawn(ChunkTask(task));
    }
}

fn poll_chunk_events(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ChunkTask)>,
    mut chunks: ResMut<Chunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    tasks
        .iter_mut()
        .filter_map(|(entity, mut task)| {
            block_on(poll_once(&mut task.0))
                .and_then(|event| {
                    event
                        .map_err(|err| error!("Error while processing chunk task: {:?}", err))
                        .ok()
                })
                .map(|event| (entity, event))
        })
        .for_each(|(entity, event)| {
            match event {
                ChunkEvent::LoadComplete(chunk, mesh) => {
                    // spawn shit mesh
                    commands.spawn((PbrBundle {
                        transform: Transform::from_translation(chunk.position.to_world()),
                        mesh: meshes.add(mesh),
                        material: materials.add(StandardMaterial::from_color(Color::BLACK)),
                        ..default()
                    },));
                    chunks.chunks.insert(chunk.position, chunk);
                }
                ChunkEvent::UnloadComplete(pos) => {
                    chunks.chunks.remove(&pos);
                    chunks.busy.remove(&pos);
                }
            }
            commands.entity(entity).despawn();
        });
}

pub async fn load_chunk(pos: ChunkPos) -> anyhow::Result<ChunkEvent> {
    let noise = noise::OpenSimplex::new(0);

    // load all neighbouring chunks
    let mut chunk = Chunk::empty(pos);
    let north = Chunk::empty(pos + ChunkPos::NORTH).filled(BlockType::Stone);
    let east = Chunk::empty(pos + ChunkPos::EAST).filled(BlockType::Stone);
    let south = Chunk::empty(pos + ChunkPos::SOUTH).filled(BlockType::Stone);
    let west = Chunk::empty(pos + ChunkPos::WEST).filled(BlockType::Stone);
    let up = Chunk::empty(pos + ChunkPos::UP).filled(BlockType::Stone);
    let down = Chunk::empty(pos + ChunkPos::DOWN).filled(BlockType::Stone);

    // generate
    chunk.generate_mut(&noise);

    // construct neighbours
    let data = ChunkNeighbours {
        chunk: &chunk,
        north: &north,
        east: &east,
        south: &south,
        west: &west,
        up: &up,
        down: &down,
    };

    // mesh
    let mesh = mesh::build(data);

    Ok(ChunkEvent::LoadComplete(chunk, mesh))
}

pub async fn unload_chunk(pos: ChunkPos) -> anyhow::Result<ChunkEvent> {
    Ok(ChunkEvent::UnloadComplete(pos))
}

pub async fn modify_block(
    pos: ChunkPos,
    block_pos: BlockPos,
    block: BlockType,
) -> anyhow::Result<ChunkEvent> {
    todo!()
}
