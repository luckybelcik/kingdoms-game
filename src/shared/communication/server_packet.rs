use std::sync::Arc;

use crate::shared::chunk::Chunk;

pub enum ServerPacket {
    Ping,
    Chunk(Arc<Chunk>),
}
