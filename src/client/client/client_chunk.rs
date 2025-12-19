use std::collections::HashMap;

use arc_swap::ArcSwap;

use crate::{
    client::client::chunk_mesh::StoredChunkMesh,
    shared::{
        chunk::Chunk,
        coordinate_systems::{chunk_pos::ChunkPos, chunk_relative::ChunkRelative},
        traits::world_inspector::WorldInspector,
    },
};

#[derive(Clone)]
pub struct ClientChunk {
    pub chunk: Chunk,
    pub mesh: StoredChunkMesh,
}

impl ClientChunk {
    pub fn new(chunk: Chunk, mesh: StoredChunkMesh) -> Self {
        Self { chunk, mesh }
    }
}

impl WorldInspector for HashMap<ChunkPos, ArcSwap<ClientChunk>> {
    fn get_block_id(&self, chunk_pos: ChunkPos, rel_pos: ChunkRelative) -> u16 {
        self.get(&chunk_pos)
            .map(|c| c.load().chunk.get_block(rel_pos))
            .unwrap_or(0)
    }
}
