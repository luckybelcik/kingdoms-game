use std::{
    sync::mpsc::{Receiver, Sender},
    time::Instant,
};

use engine_core::{chunk_pos::ChunkPos, entity_pos::EntityPos};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct PlayerData {
    pub player_permissions: PlayerPermissions,
    pub name: String,
    pub position: EntityPos,
    pub chunk_tick_position: ChunkPos,
    pub visible_chunks: FxHashSet<ChunkPos>,
    pub chunks_awaiting_generation: FxHashSet<ChunkPos>,
    pub connection_type: ConnectionType,
    pub last_ping: Instant,
    pub render_distance: u8,
    pub selected_block: u16,
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
            render_distance: self.render_distance,
            selected_block: self.selected_block,
        }
    }

    pub fn to_client_data(&self) -> ClientPlayerData {
        ClientPlayerData {
            player_permissions: self.player_permissions.clone(),
            name: self.name.clone(),
            position: self.position.clone(),
            render_distance: self.render_distance,
            selected_block: 1,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ClientPlayerData {
    pub player_permissions: PlayerPermissions,
    pub name: String,
    pub position: EntityPos,
    pub render_distance: u8,
    pub selected_block: u16,
}

impl ClientPlayerData {
    pub fn log_desync(data_1: &ClientPlayerData, data_2: &ClientPlayerData) {
        if data_1.name != data_2.name {
            eprintln!(
                "DESYNC: Names desynced (1: {}) (2: {})",
                data_1.name, data_2.name
            );
        }
        if data_1.player_permissions != data_2.player_permissions {
            eprintln!(
                "DESYNC: Permissions desynced (1: {:?}) (2: {:?})",
                data_1.player_permissions, data_2.player_permissions
            );
        }
        if data_1.position != data_2.position {
            eprintln!(
                "DESYNC: Position desynced (1: {:?}) (2: {:?})",
                data_1.position, data_2.position
            );
        }
        if data_1.render_distance != data_2.render_distance {
            eprintln!(
                "DESYNC: Render distance desynced (1: {}) (2: {})",
                data_1.render_distance, data_2.render_distance
            );
        }
    }
}

#[derive(Debug)]
pub enum ConnectionType {
    Local(Sender<Vec<u8>>, Receiver<Vec<u8>>),
    Remote,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SendableConnectionType {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendablePlayerData {
    pub name: String,
    pub position: EntityPos,
    pub chunk_tick_position: ChunkPos,
    pub connection_type: SendableConnectionType,
    pub render_distance: u8,
    pub selected_block: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
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
