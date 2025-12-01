#[derive(Debug)]
pub struct ChunkDrawCallInfo {
    pub buffer_offset: u64,
    pub instance_count: u64,
    pub visible: bool,
}