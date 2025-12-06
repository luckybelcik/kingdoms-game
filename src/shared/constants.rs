pub const CHUNK_SIZE: usize = 64;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_POS_BITS: usize = 6;
pub type ChunkBitRow = u64;
