use std::{
    collections::HashSet,
    sync::mpsc::{Receiver, Sender},
    time::Instant,
};

use crate::shared::{
    communication::{client_packet::ClientPacket, server_packet::ServerPacket},
    coordinate_systems::{chunk_pos::ChunkPos, entity_pos::EntityPos},
};

#[derive(Debug)]
pub struct PlayerData {
    pub player_permissions: PlayerPermissions,
    pub name: String,
    pub position: EntityPos,
    pub chunk_tick_position: ChunkPos,
    pub visible_chunks: HashSet<ChunkPos>,
    pub chunks_awaiting_generation: HashSet<ChunkPos>,
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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PlayerPermissions {
    None = 0,   // basically no permissions
    Helper = 1, // non-destructive permissions
    Admin = 2,  // destructive permissions
}

impl PlayerPermissions {
    pub fn at_least_helper(&self) -> bool {
        match self {
            PlayerPermissions::None => false,
            PlayerPermissions::Helper => true,
            PlayerPermissions::Admin => true,
        }
    }

    pub fn is_admin(&self) -> bool {
        match self {
            PlayerPermissions::None => false,
            PlayerPermissions::Helper => false,
            PlayerPermissions::Admin => true,
        }
    }
}
