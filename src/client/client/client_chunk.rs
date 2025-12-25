use std::sync::Arc;

use arc_swap::ArcSwap;
use rustc_hash::FxHashMap;

use crate::{
    client::client::chunk_mesh::StoredChunkMesh,
    shared::{
        chunk::Chunk,
        coordinate_systems::{chunk_pos::ChunkPos, chunk_relative::ChunkRelative},
        traits::world_inspector::WorldInspector,
    },
};

pub struct ClientChunk {
    pub chunk: ArcSwap<Chunk>,
    pub mesh: ArcSwap<StoredChunkMesh>,
}

impl ClientChunk {
    pub fn new(chunk: Chunk, mesh: StoredChunkMesh) -> Self {
        Self {
            chunk: ArcSwap::new(Arc::new(chunk)),
            mesh: ArcSwap::new(Arc::new(mesh)),
        }
    }

    pub fn new_prewrapped(chunk: ArcSwap<Chunk>, mesh: ArcSwap<StoredChunkMesh>) -> Self {
        Self { chunk, mesh }
    }
}

impl WorldInspector for FxHashMap<ChunkPos, ClientChunk> {
    fn get_block_id(&self, chunk_pos: ChunkPos, rel_pos: ChunkRelative) -> u16 {
        self.get(&chunk_pos)
            .map(|c| c.chunk.load().get_block(rel_pos))
            .unwrap_or(0)
    }
}
