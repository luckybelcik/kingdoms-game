use std::sync::Arc;

use crate::shared::{chunk::Chunk, communication::player_data::SendablePlayerData};

pub enum ServerPacket {
    Ping,
    Chunk(Arc<Chunk>),
    Debug(Box<SendablePlayerData>),
}
