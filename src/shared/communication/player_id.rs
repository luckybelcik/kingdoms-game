use serde::{Deserialize, Serialize};

#[derive(Debug, Hash, PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerId {
    id: u64,
}

impl PlayerId {
    pub fn new() -> Self {
        PlayerId { id: rand::random() }
    }
}
