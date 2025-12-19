use crate::client::app::app::App;
use std::collections::HashMap;

use arc_swap::ArcSwap;

use crate::{client::client::mesher::Mesher, shared::coordinate_systems::chunk_pos::ChunkPos};

pub struct Client {
    pub chunks: HashMap<ChunkPos, ArcSwap<ClientChunk>>,
    pub dirty_chunks: HashSet<ChunkPos>,
    pub mesher: Mesher,
    pub app: App,
}

impl Client {
    pub fn create() -> Client {}
}
