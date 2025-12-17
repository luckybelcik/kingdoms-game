use std::{
    sync::mpsc::{Receiver, Sender},
    time::Instant,
};

use crate::shared::{
    communication::{client_packet::ClientPacket, server_packet::ServerPacket},
    coordinate_systems::{chunk_pos::ChunkPos, entity_pos::EntityPos},
};

pub struct PlayerData {
    pub name: String,
    pub position: EntityPos,
    pub chunk_tick_position: ChunkPos,
    pub connection_type: ConnectionType,
    pub last_ping: Instant,
    pub render_distance: u8,
}

pub enum ConnectionType {
    Local(Sender<ServerPacket>, Receiver<ClientPacket>),
    Remote,
}
