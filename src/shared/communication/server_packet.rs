use std::sync::Arc;

use crate::shared::{chunk::Chunk, communication::player_data::SendablePlayerData};

pub enum ServerPacket {
    Ping,
    Chunk(Arc<Chunk>),
    DebugPlayer(Box<SendablePlayerData>),
    DebugChunk(Box<DebugChunkData>),
}

#[derive(Debug, Clone, Copy)]
pub struct DebugChunkData {
    pub chunk_count: u32,
    pub dirty_chunks: u32,
    pub generating_chunks: u32,
}
