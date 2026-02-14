use std::sync::Arc;

use arc_swap::ArcSwap;
use engine_core::{chunk_pos::ChunkPos, chunk_relative::ChunkRelative};
use engine_world::chunk::{Chunk, WorldInspector};
use rustc_hash::FxHashMap;

use crate::client::chunk_mesh::StoredChunkMesh;

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

pub struct ClientWorld {
    pub chunks: FxHashMap<ChunkPos, ClientChunk>,
}

impl WorldInspector for ClientWorld {
    fn get_block_id(&self, chunk_pos: ChunkPos, rel_pos: ChunkRelative) -> u16 {
        self.chunks
            .get(&chunk_pos)
            .map(|c| c.chunk.load().get_block(rel_pos))
            .unwrap_or(0)
    }
}
