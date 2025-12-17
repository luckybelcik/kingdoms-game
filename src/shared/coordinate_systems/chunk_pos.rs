use bytemuck::{Pod, Zeroable};

use crate::shared::coordinate_systems::block_pos::BlockPos;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Zeroable, Pod)]
pub struct ChunkPos(nalgebra_glm::IVec3);

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkPos(nalgebra_glm::IVec3::new(x, y, z))
    }
}

impl From<BlockPos> for ChunkPos {
    fn from(pos: BlockPos) -> Self {
        pos.to_chunk_pos()
    }
}

impl std::ops::Deref for ChunkPos {
    type Target = nalgebra_glm::IVec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
