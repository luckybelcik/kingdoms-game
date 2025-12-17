use crate::{
    client::rendering::chunk_mesh::StoredChunkMesh,
    shared::{chunk::Chunk},
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
