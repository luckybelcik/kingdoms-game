use crate::{
    chunk_pos::ChunkPos, chunk_relative::ChunkRelative, constants::CHUNK_SIZE,
    entity_pos::EntityPos,
};

/// Represents a block position in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos(nalgebra_glm::IVec3);

impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        BlockPos(nalgebra_glm::IVec3::new(x, y, z))
    }

    pub fn to_chunk_relative(&self) -> ChunkRelative {
        ChunkRelative::new(
            self.x.rem_euclid(CHUNK_SIZE as i32) as u8,
            self.y.rem_euclid(CHUNK_SIZE as i32) as u8,
            self.z.rem_euclid(CHUNK_SIZE as i32) as u8,
        )
    }

    pub fn to_chunk_pos(&self) -> ChunkPos {
        ChunkPos::new(
            self.x.div_floor(CHUNK_SIZE as i32),
            self.y.div_floor(CHUNK_SIZE as i32),
            self.z.div_floor(CHUNK_SIZE as i32),
        )
    }
}

impl From<EntityPos> for BlockPos {
    fn from(pos: EntityPos) -> Self {
        pos.to_block_pos()
    }
}

impl std::ops::Deref for BlockPos {
    type Target = nalgebra_glm::IVec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BlockPos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
