use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};

use arc_swap::ArcSwap;
use nalgebra_glm::IVec3;

use crate::shared::{
    chunk::Chunk,
    communication::{
        client_packet::{ClientAction, ClientPacket},
        player_data::{ConnectionType, PlayerData},
        player_id::PlayerId,
        server_packet::ServerPacket,
    },
    coordinate_systems::{chunk_pos::ChunkPos, entity_pos::EntityPos},
};

pub struct Server {
    pub chunks: HashMap<ChunkPos, ArcSwap<Chunk>>,
    pub dirty_chunks: HashSet<ChunkPos>,
    pub players: HashMap<PlayerId, PlayerData>,
    pub tick: u128,
}

impl Server {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            dirty_chunks: HashSet::new(),
            players: HashMap::new(),
            tick: 0,
        }
    }

    pub fn add_local_player(
        &mut self,
        player_id: PlayerId,
        server_sender: Sender<ServerPacket>,
        client_receiver: Receiver<ClientPacket>,
    ) {
        let player_data = PlayerData {
            name: "Local".to_string(),
            position: EntityPos::new(0.0, 0.0, 0.0),
            chunk_tick_position: ChunkPos::new(0, 500, 0),
            connection_type: ConnectionType::Local(server_sender, client_receiver),
            last_ping: Instant::now(),
            render_distance: 3,
        };
        self.players.insert(player_id, player_data);
    }

    pub fn handle_client_packet(&mut self, client_packet: ClientPacket) {
        let player_id = client_packet.player_id;
        match client_packet.action {
            ClientAction::Ping => {
                if let Some(player_data) = self.players.get_mut(&player_id) {
                    player_data.last_ping = Instant::now();
                }
            }
        }
    }

    pub fn load_chunks(&mut self) {
        for player_data in self.players.values_mut() {
            let player_chunk_pos = player_data.position.to_block_pos().to_chunk_pos();
            if player_chunk_pos != player_data.chunk_tick_position {
                let current_nearby_chunks =
                    get_chunks_in_radius(player_chunk_pos, player_data.render_distance);
                let old_nearby_chunks = get_chunks_in_radius(
                    player_data.chunk_tick_position,
                    player_data.render_distance,
                );

                let to_load: Vec<ChunkPos> = current_nearby_chunks
                    .difference(&old_nearby_chunks)
                    .cloned()
                    .collect();
                let to_unload: Vec<ChunkPos> = old_nearby_chunks
                    .difference(&current_nearby_chunks)
                    .cloned()
                    .collect();

                for chunk_pos in to_load {
                    Self::load_chunk(
                        &mut self.chunks,
                        &mut self.dirty_chunks,
                        player_data,
                        chunk_pos,
                    );
                }
                for chunk_pos in to_unload {
                    Self::unload_chunk(&mut self.chunks, &mut self.dirty_chunks, chunk_pos);
                }
            }
            player_data.chunk_tick_position = player_data.position.to_block_pos().to_chunk_pos();
        }
    }

    fn load_chunk(
        chunks: &mut HashMap<ChunkPos, ArcSwap<Chunk>>,
        dirty_chunks: &mut HashSet<ChunkPos>,
        player_data: &PlayerData,
        chunk_pos: ChunkPos,
    ) {
        if let Some(chunk) = chunks.get(&chunk_pos) {
            let chunk_arc = chunk.load_full();
            Self::send_packet(player_data, ServerPacket::Chunk(chunk_arc.clone()));
            return;
        }

        let chunk = Arc::new(Chunk::generate(chunk_pos, dirty_chunks));
        Self::send_packet(player_data, ServerPacket::Chunk(Arc::new((*chunk).clone())));
        chunks.insert(chunk_pos, ArcSwap::new(chunk));
    }

    fn unload_chunk(
        chunks: &mut HashMap<ChunkPos, ArcSwap<Chunk>>,
        dirty_chunks: &mut HashSet<ChunkPos>,
        chunk_pos: ChunkPos,
    ) {
        if chunks.contains_key(&chunk_pos) {
            chunks.remove(&chunk_pos);
        }

        if dirty_chunks.contains(&chunk_pos) {
            dirty_chunks.remove(&chunk_pos);
        }
    }

    fn send_packet(player_data: &PlayerData, server_packet: ServerPacket) {
        match &player_data.connection_type {
            ConnectionType::Local(server_packet_sender, _) => {
                server_packet_sender
                    .send(server_packet)
                    .expect("Failed to send server packet");
            }
            ConnectionType::Remote => {
                unimplemented!("cant send remotely yet");
            }
        }
    }

    pub fn update(&mut self) {
        self.tick += 1;
    }
}

pub fn get_chunks_in_radius(player_chunk_pos: ChunkPos, radius: u8) -> HashSet<ChunkPos> {
    let mut nearby_chunks = HashSet::new();
    let radius = radius as i32;
    let radius_sq = (radius * radius) as f32;

    for x in -radius..=radius {
        for y in -radius..=radius {
            for z in -radius..=radius {
                let offset = IVec3::new(x, y, z);

                if (x * x + y * y + z * z) as f32 <= radius_sq {
                    nearby_chunks.insert(ChunkPos::new_from_vec(*player_chunk_pos + offset));
                }
            }
        }
    }

    nearby_chunks
}
