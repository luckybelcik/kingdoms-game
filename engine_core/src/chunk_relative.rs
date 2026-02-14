use crate::{block_pos::BlockPos, constants::CHUNK_SIZE};

/// Represents a chunk relative block positio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkRelative(nalgebra_glm::U8Vec3);

impl ChunkRelative {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        ChunkRelative(nalgebra_glm::U8Vec3::new(x, y, z))
    }

    pub fn to_array_index(&self) -> usize {
        self.x as usize + self.y as usize * CHUNK_SIZE + self.z as usize * CHUNK_SIZE * CHUNK_SIZE
    }
}

impl From<BlockPos> for ChunkRelative {
    fn from(world_relative: BlockPos) -> Self {
        world_relative.to_chunk_relative()
    }
}

impl std::ops::Deref for ChunkRelative {
    type Target = nalgebra_glm::U8Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
