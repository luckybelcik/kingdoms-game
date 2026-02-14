use engine_core::chunk_pos::ChunkPos;

#[derive(PartialEq, Eq)]
pub struct PrioritizedJob {
    pub priority: i32, // Lower value = higher priority
    pub pos: ChunkPos,
}

impl Ord for PrioritizedJob {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for PrioritizedJob {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
