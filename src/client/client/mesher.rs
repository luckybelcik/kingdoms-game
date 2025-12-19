use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use arc_swap::ArcSwap;

use crate::{
    client::client::{
        chunk_mesh::{MeshJob, SendableChunkMesh},
        client_chunk::ClientChunk,
    },
    shared::coordinate_systems::chunk_pos::ChunkPos,
};

pub struct Mesher {
    job_sender: Sender<MeshJob>,
    mesh_receiver: Receiver<SendableChunkMesh>,
}

impl Mesher {
    pub fn create() -> Self {
        let (job_sender, job_receiver) = std::sync::mpsc::channel::<MeshJob>();
        let (mesh_sender, mesh_receiver) = std::sync::mpsc::channel::<SendableChunkMesh>();

        std::thread::spawn(move || {
            for job in job_receiver {
                let sendable = SendableChunkMesh::make_mesh(&job);
                mesh_sender.send(sendable).unwrap();
            }
        });

        return Mesher {
            job_sender,
            mesh_receiver,
        };
    }

    pub fn upload_for_remeshing(
        &self,
        dirty_keys: &mut HashSet<ChunkPos>,
        chunks: &mut HashMap<ChunkPos, ArcSwap<ClientChunk>>,
    ) {
        for key in dirty_keys.iter() {
            if let Some(chunk) = chunks.get(key) {
                let chunk_pos_right = ChunkPos::new(key.x + 1, key.y, key.z);
                let chunk_pos_left = ChunkPos::new(key.x - 1, key.y, key.z);
                let chunk_pos_up = ChunkPos::new(key.x, key.y + 1, key.z);
                let chunk_pos_down = ChunkPos::new(key.x, key.y - 1, key.z);
                let chunk_pos_forward = ChunkPos::new(key.x, key.y, key.z + 1);
                let chunk_pos_backward = ChunkPos::new(key.x, key.y, key.z - 1);

                let nearby_chunks: [Option<Arc<ClientChunk>>; 6] = [
                    chunks.get(&chunk_pos_right).map(|c| c.load_full()),
                    chunks.get(&chunk_pos_left).map(|c| c.load_full()),
                    chunks.get(&chunk_pos_up).map(|c| c.load_full()),
                    chunks.get(&chunk_pos_down).map(|c| c.load_full()),
                    chunks.get(&chunk_pos_forward).map(|c| c.load_full()),
                    chunks.get(&chunk_pos_backward).map(|c| c.load_full()),
                ];

                let loaded_chunk = chunk.load_full();

                self.job_sender.send((loaded_chunk, nearby_chunks)).unwrap();
            }
        }

        dirty_keys.clear();
    }

    pub fn receive_from_remeshing(&self) -> Vec<SendableChunkMesh> {
        let mut new_meshes = Vec::new();

        for sent_mesh in self.mesh_receiver.try_iter() {
            new_meshes.push(sent_mesh);
        }

        new_meshes
    }
}
