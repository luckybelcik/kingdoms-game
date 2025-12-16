use crate::{client::rendering::chunk_mesh::StoredChunkMesh, shared::chunk::Chunk};

#[derive(Clone)]
pub struct ClientChunk {
    pub chunk: Chunk,
    pub mesh: StoredChunkMesh,
}

impl ClientChunk {
    pub fn new(chunk: Chunk, mesh: StoredChunkMesh) -> Self {
        Self { chunk, mesh }
    }

    pub fn new_full(chunk_pos: nalgebra_glm::IVec3) -> Self {
        let chunk = Chunk::new_full(chunk_pos.x, chunk_pos.y, chunk_pos.z);
        let mesh = StoredChunkMesh::new_empty();
        Self { chunk, mesh }
    }
}
