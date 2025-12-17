use crate::{
    client::rendering::chunk_mesh::StoredChunkMesh,
    shared::{chunk::Chunk, coordinate_systems::chunk_pos::ChunkPos},
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

    pub fn new_full(chunk_pos: ChunkPos) -> Self {
        let chunk = Chunk::new_full(chunk_pos);
        let mesh = StoredChunkMesh::new_empty();
        Self { chunk, mesh }
    }
}
