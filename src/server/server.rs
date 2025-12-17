use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};

use arc_swap::ArcSwap;

use crate::shared::{
    chunk::Chunk,
    communication::{
        client_packet::{ClientAction, ClientPacket},
        player_data::{ConnectionType, PlayerData},
        player_id::PlayerId,
        server_packet::ServerPacket,
    },
};

pub struct Server {
    pub chunks: HashMap<nalgebra_glm::IVec3, ArcSwap<Chunk>>,
    pub dirty_chunks: HashSet<nalgebra_glm::IVec3>,
    pub players: HashMap<PlayerId, PlayerData>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            dirty_chunks: HashSet::new(),
            players: HashMap::new(),
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
            position: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            connection_type: ConnectionType::Local(server_sender, client_receiver),
            last_ping: Instant::now(),
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

    pub fn load_chunk(&mut self) {}
}
