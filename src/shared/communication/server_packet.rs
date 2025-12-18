use std::sync::Arc;

use crate::shared::{chunk::Chunk, communication::player_data::PlayerData};

pub enum ServerPacket {
    Ping,
    Chunk(Arc<Chunk>),
    Debug(Box<PlayerData>),
}
