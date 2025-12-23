use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientKeybindableActions {
    BreakBlock,
    PlaceBlock,
    MoveForwards,
    MoveBackwards,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    RotateUp,
    RotateDown,
    RotateLeft,
    RotateRight,
    RequestServerPlayerData,
    RequestServerChunkInfo,
}

impl ClientKeybindableActions {
    pub fn is_single_press(&self) -> bool {
        match self {
            Self::BreakBlock => true,
            Self::PlaceBlock => true,
            Self::MoveForwards => false,
            Self::MoveBackwards => false,
            Self::MoveLeft => false,
            Self::MoveRight => false,
            Self::MoveUp => false,
            Self::MoveDown => false,
            Self::RotateUp => false,
            Self::RotateDown => false,
            Self::RotateLeft => false,
            Self::RotateRight => false,
            Self::RequestServerChunkInfo => true,
            Self::RequestServerPlayerData => true,
        }
    }

    pub fn is_holdable(&self) -> bool {
        !self.is_single_press()
    }
}
