use arc_swap::ArcSwap;
use engine_core::{
    chunk_pos::ChunkPos,
    chunk_relative::ChunkRelative,
    constants::{CHUNK_SIZE, CHUNK_VOLUME, ChunkBitRow},
};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
                    let b_y = y as i32 + c_y * CHUNK_SIZE as i32;

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
        if x >= CHUNK_SIZE as u8 || y >= CHUNK_SIZE as u8 || z >= CHUNK_SIZE as u8 {
            return;
        }

        let x_u = x as usize;
        let y_u = y as usize;
        let z_u = z as usize;

        let block_idx = x_u + (y_u * CHUNK_SIZE) + (z_u * CHUNK_SIZE * CHUNK_SIZE);
        let mask_idx_a = y_u + (z_u * CHUNK_SIZE);
        let mask_idx_b = y_u + (x_u * CHUNK_SIZE);

        unsafe {
            *self.blocks.get_unchecked_mut(block_idx) = block;

            let is_not_air = (block != 0) as ChunkBitRow;

            let bit_x = (1 as ChunkBitRow) << x;
            let mask_a = self.chunk_mask.get_unchecked_mut(mask_idx_a);
            *mask_a = (*mask_a & !bit_x) | (is_not_air.wrapping_neg() & bit_x);

            let bit_z = (1 as ChunkBitRow) << z;
            let mask_b = self.xz_swap_chunk_mask.get_unchecked_mut(mask_idx_b);
            *mask_b = (*mask_b & !bit_z) | (is_not_air.wrapping_neg() & bit_z);
        }
    }

    pub fn set_block(
        &mut self,
        pos: ChunkRelative,
        block: u16,
        dirty_chunks: &mut FxHashSet<ChunkPos>,
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

            if pos.x == CHUNK_SIZE as u8 - 1 {
                dirty_chunks.insert(self.chunk_pos.offset_copy(1, 0, 0));
            }
            if pos.x == 0 {
                dirty_chunks.insert(self.chunk_pos.offset_copy(-1, 0, 0));
            }
            if pos.y == CHUNK_SIZE as u8 - 1 {
                dirty_chunks.insert(self.chunk_pos.offset_copy(0, 1, 0));
            }
            if pos.y == 0 {
                dirty_chunks.insert(self.chunk_pos.offset_copy(0, -1, 0));
            }
            if pos.z == CHUNK_SIZE as u8 - 1 {
                dirty_chunks.insert(self.chunk_pos.offset_copy(0, 0, 1));
            }
            if pos.z == 0 {
                dirty_chunks.insert(self.chunk_pos.offset_copy(0, 0, -1));
            }
        }
    }

    pub fn get_block(&self, pos: ChunkRelative) -> u16 {
        if pos.x >= CHUNK_SIZE as u8 || pos.y >= CHUNK_SIZE as u8 || pos.z >= CHUNK_SIZE as u8 {
            0 // treat out-of-bounds as air
        } else {
            self.blocks[pos.to_array_index()]
        }
    }

    pub fn get_block_unsafe(&self, index: usize) -> u16 {
        self.blocks[index]
    }

    pub fn get_chunk_mask(&self) -> &[ChunkBitRow] {
        &self.chunk_mask
    }
}

pub trait WorldInspector {
    fn get_block_id(&self, chunk_pos: ChunkPos, rel_pos: ChunkRelative) -> u16;
}

impl WorldInspector for FxHashMap<ChunkPos, ArcSwap<Chunk>> {
    fn get_block_id(&self, chunk_pos: ChunkPos, rel_pos: ChunkRelative) -> u16 {
        self.get(&chunk_pos)
            .map(|c| c.load().get_block(rel_pos))
            .unwrap_or(0)
    }
}
