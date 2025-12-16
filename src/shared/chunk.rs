use std::collections::HashSet;

use crate::shared::constants::{CHUNK_SIZE, CHUNK_VOLUME, ChunkBitRow};
use nalgebra_glm as glm;

#[derive(Clone)]
pub struct Chunk {
    chunk_pos: glm::IVec3,
    blocks: [u16; CHUNK_VOLUME],
    pub chunk_mask: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE],
    pub xz_swap_chunk_mask: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    pub fn get_chunk_pos(&self) -> glm::IVec3 {
        self.chunk_pos
    }

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [0; CHUNK_VOLUME],
            chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE],
            xz_swap_chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    pub fn new_full(x: i32, y: i32, z: i32) -> Self {
        Self {
            chunk_pos: glm::vec3(x, y, z),
            blocks: [1; CHUNK_VOLUME],
            chunk_mask: [(!0); CHUNK_SIZE * CHUNK_SIZE],
            xz_swap_chunk_mask: [(!0); CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    pub fn set_block(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: u16,
        dirty_chunks: &mut HashSet<glm::IVec3>,
    ) {
        if x < CHUNK_SIZE || y < CHUNK_SIZE || z < CHUNK_SIZE {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE] = block;
            if block == 0 {
                self.chunk_mask[y + z * CHUNK_SIZE] &= !(1 << x);
                self.xz_swap_chunk_mask[y + x * CHUNK_SIZE] &= !(1 << z);
            } else {
                self.chunk_mask[y + z * CHUNK_SIZE] |= 1 << x;
                self.xz_swap_chunk_mask[y + x * CHUNK_SIZE] |= 1 << z;
            }

            dirty_chunks.insert(self.chunk_pos);
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            0 // treat out-of-bounds as air
        } else {
            self.blocks[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE]
        }
    }

    pub fn get_chunk_mask(&self) -> &[ChunkBitRow] {
        &self.chunk_mask
    }
}
