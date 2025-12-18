use std::{
    sync::mpsc::{Receiver, Sender},
    time::Instant,
};

use crate::shared::{
    communication::{client_packet::ClientPacket, server_packet::ServerPacket},
    coordinate_systems::{chunk_pos::ChunkPos, entity_pos::EntityPos},
};

#[derive(Debug)]
pub struct PlayerData {
    pub name: String,
    pub position: EntityPos,
    pub chunk_tick_position: ChunkPos,
    pub connection_type: ConnectionType,
    pub last_ping: Instant,
    pub render_distance: u8,
}

impl PlayerData {
    pub fn to_sendable(&self) -> SendablePlayerData {
        SendablePlayerData {
            name: self.name.clone(),
            position: self.position,
            chunk_tick_position: self.chunk_tick_position,
            connection_type: match self.connection_type {
                ConnectionType::Local(_, _) => SendableConnectionType::Local,
                ConnectionType::Remote => SendableConnectionType::Remote,
            },
            last_ping: self.last_ping,
            render_distance: self.render_distance,
        }
    }
}

#[derive(Debug)]
pub enum ConnectionType {
    Local(Sender<ServerPacket>, Receiver<ClientPacket>),
    Remote,
}

#[derive(Debug, Clone, Copy)]
pub enum SendableConnectionType {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct SendablePlayerData {
    pub name: String,
    pub position: EntityPos,
    pub chunk_tick_position: ChunkPos,
    pub connection_type: SendableConnectionType,
    pub last_ping: Instant,
    pub render_distance: u8,
}
