use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::shared::constants::CHUNK_SIZE;
use crate::shared::coordinate_systems::{block_pos::BlockPos, chunk_relative::ChunkRelative};

#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Zeroable,
    Pod,
    Serialize,
    Deserialize
)]
pub struct ChunkPos(nalgebra_glm::IVec3);

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkPos(nalgebra_glm::IVec3::new(x, y, z))
    }

    pub fn new_from_vec(vec: nalgebra_glm::IVec3) -> Self {
        ChunkPos(vec)
    }

    pub fn offset(&mut self, x: i32, y: i32, z: i32) {
        self.x = self.x + x;
        self.y = self.y + y;
        self.z = self.z + z;
    }

    pub fn offset_copy(&self, x: i32, y: i32, z: i32) -> Self {
        ChunkPos::new(self.x + x, self.y + y, self.z + z)
    }

    pub fn to_block_pos(&self, chunk_relative: ChunkRelative) -> BlockPos {
        BlockPos::new(
            self.x * CHUNK_SIZE as i32 + chunk_relative.x as i32,
            self.y * CHUNK_SIZE as i32 + chunk_relative.y as i32,
            self.z * CHUNK_SIZE as i32 + chunk_relative.z as i32,
        )
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

impl std::ops::DerefMut for ChunkPos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
