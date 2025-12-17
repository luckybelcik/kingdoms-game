#[derive(Debug, Hash, PartialEq, PartialOrd, Ord, Eq, Clone)]
pub struct PlayerId {
    id: u64,
}

impl PlayerId {
    pub fn new() -> Self {
        PlayerId { id: rand::random() }
    }
}
