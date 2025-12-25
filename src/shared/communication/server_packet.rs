use serde::{Deserialize, Serialize};

use crate::shared::{
    chunk::Chunk,
    communication::player_data::{ClientPlayerData, SendablePlayerData},
};

#[derive(Serialize, Deserialize)]
pub enum ServerPacket {
    Ping,
    Chunk(Box<Chunk>),
    PlayerData(ClientPlayerData),
    DebugPlayer(Box<SendablePlayerData>),
    DebugChunk(Box<DebugChunkData>),
    Denial(DenialReason),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DebugChunkData {
    pub chunk_count: u32,
    pub dirty_chunks: u32,
    pub generating_chunks: u32,
}

#[derive(Serialize, Deserialize)]
pub enum DenialReason {
    InsufficientPermissions,
}

impl DenialReason {
    pub fn message(&self) -> &'static str {
        match self {
            Self::InsufficientPermissions => {
                "You don't have the permissions to request this packet."
            }
        }
    }
}
