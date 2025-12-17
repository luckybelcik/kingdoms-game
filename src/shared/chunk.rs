use std::collections::HashSet;

use crate::shared::{
    constants::{CHUNK_SIZE, CHUNK_VOLUME, ChunkBitRow},
    coordinate_systems::{chunk_pos::ChunkPos, chunk_relative::ChunkRelative},
};

#[derive(Clone, Debug)]
pub struct Chunk {
    chunk_pos: ChunkPos,
    blocks: Vec<u16>,
    pub chunk_mask: Vec<ChunkBitRow>,
    pub xz_swap_chunk_mask: Vec<ChunkBitRow>,
}

impl Chunk {
    pub fn get_chunk_pos(&self) -> ChunkPos {
        self.chunk_pos
    }

    fn new(chunk_pos: ChunkPos) -> Self {
        Self {
            chunk_pos,
            blocks: vec![0; CHUNK_VOLUME],
            chunk_mask: vec![0; CHUNK_SIZE * CHUNK_SIZE],
            xz_swap_chunk_mask: vec![0; CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    // we avoid using the fancy coordinate types here cause its a hot loop
    // TRUST MI BRO this is 100x faster
    pub fn generate(chunk_pos: ChunkPos) -> Self {
        let mut chunk = Self::new(chunk_pos);

        let c_y = chunk_pos.y;

        for x in 0..CHUNK_SIZE as u8 {
            for y in 0..CHUNK_SIZE as u8 {
                for z in 0..CHUNK_SIZE as u8 {
                    let b_y = y as i32 + c_y;

                    if b_y < 0 {
                        let block = 1;
                        chunk.set_block_unsafe(x, y, z, block);
                    }
                }
            }
        }

        chunk
    }

    // unsafe version without the fancy types
    fn set_block_unsafe(&mut self, x: u8, y: u8, z: u8, block: u16) {
        if x < CHUNK_SIZE as u8 || y < CHUNK_SIZE as u8 || z < CHUNK_SIZE as u8 {
            self.blocks
                [x as usize + y as usize * CHUNK_SIZE + z as usize * CHUNK_SIZE * CHUNK_SIZE] =
                block;
            if block == 0 {
                self.chunk_mask[y as usize + z as usize * CHUNK_SIZE] &= !(1 << x);
                self.xz_swap_chunk_mask[y as usize + x as usize * CHUNK_SIZE] &= !(1 << z);
            } else {
                self.chunk_mask[y as usize + z as usize * CHUNK_SIZE] |= 1 << x;
                self.xz_swap_chunk_mask[y as usize + x as usize * CHUNK_SIZE] |= 1 << z;
            }
        }
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
