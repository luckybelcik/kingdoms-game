use std::collections::HashSet;

use crate::shared::{
    constants::{CHUNK_SIZE, CHUNK_VOLUME, ChunkBitRow},
    coordinate_systems::{chunk_pos::ChunkPos, chunk_relative::ChunkRelative},
};

#[derive(Clone, Debug)]
pub struct Chunk {
    chunk_pos: ChunkPos,
    blocks: [u16; CHUNK_VOLUME],
    pub chunk_mask: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE],
    pub xz_swap_chunk_mask: [ChunkBitRow; CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    pub fn get_chunk_pos(&self) -> ChunkPos {
        self.chunk_pos
    }

    fn new(chunk_pos: ChunkPos) -> Self {
        Self {
            chunk_pos,
            blocks: [0; CHUNK_VOLUME],
            chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE],
            xz_swap_chunk_mask: [0; CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    pub fn generate(chunk_pos: ChunkPos, dirty_chunks: &mut HashSet<ChunkPos>) -> Self {
        let mut chunk = Self::new(chunk_pos);

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let relative_pos = ChunkRelative::new(x as u8, y as u8, z as u8);
                    let block_pos = chunk_pos.to_block_pos(relative_pos);

                    if block_pos.y > 0 {
                        let block = 1;
                        chunk.set_block(relative_pos, block, dirty_chunks);
                    }
                }
            }
        }

        chunk
    }

    pub fn set_block(
        &mut self,
        pos: ChunkRelative,
        block: u16,
        dirty_chunks: &mut HashSet<ChunkPos>,
    ) {
        if pos.x < CHUNK_SIZE as u8 || pos.y < CHUNK_SIZE as u8 || pos.z < CHUNK_SIZE as u8 {
            self.blocks[pos.to_array_index()] = block;
            if block == 0 {
                self.chunk_mask[pos.y as usize + pos.z as usize * CHUNK_SIZE] &= !(1 << pos.x);
                self.xz_swap_chunk_mask[pos.y as usize + pos.x as usize * CHUNK_SIZE] &=
                    !(1 << pos.z);
            } else {
                self.chunk_mask[pos.y as usize + pos.z as usize * CHUNK_SIZE] |= 1 << pos.x;
                self.xz_swap_chunk_mask[pos.y as usize + pos.x as usize * CHUNK_SIZE] |= 1 << pos.z;
            }

            dirty_chunks.insert(self.chunk_pos);
        }
    }

    pub fn get_block(&self, pos: ChunkRelative) -> u16 {
        if pos.x >= CHUNK_SIZE as u8 || pos.y >= CHUNK_SIZE as u8 || pos.z >= CHUNK_SIZE as u8 {
            0 // treat out-of-bounds as air
        } else {
            self.blocks[pos.to_array_index()]
        }
    }

    pub fn get_chunk_mask(&self) -> &[ChunkBitRow] {
        &self.chunk_mask
    }
}
