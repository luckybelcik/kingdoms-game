#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockDataRender {
    id: u16,
    ao: u8,
}

impl BlockDataRender {
    pub fn new(id: u16, ao: u8) -> Self {
        Self { id, ao }
    }

    pub fn empty() -> Self {
        Self { id: 0, ao: 0 }
    }
}