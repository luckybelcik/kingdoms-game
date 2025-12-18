use crate::shared::coordinate_systems::{chunk_pos::ChunkPos, chunk_relative::ChunkRelative};

pub trait WorldInspector {
    fn get_block_id(&self, chunk_pos: ChunkPos, rel_pos: ChunkRelative) -> u16;
}
