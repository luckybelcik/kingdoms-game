use engine_core::entity_pos::EntityPos;
use nalgebra_glm::Vec3;
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
    ScrollHotbarRight,
    ScrollHotbarLeft,
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
            Self::ScrollHotbarRight => true,
            Self::ScrollHotbarLeft => true,
            Self::RequestServerChunkInfo => true,
            Self::RequestServerPlayerData => true,
        }
    }

    pub fn is_holdable(&self) -> bool {
        !self.is_single_press()
    }

    pub fn is_tickrate_independent(&self) -> bool {
        match self {
            Self::BreakBlock => false,
            Self::PlaceBlock => false,
            Self::MoveForwards => false,
            Self::MoveBackwards => false,
            Self::MoveLeft => false,
            Self::MoveRight => false,
            Self::MoveUp => false,
            Self::MoveDown => false,
            Self::RotateUp => true,
            Self::RotateDown => true,
            Self::RotateLeft => true,
            Self::RotateRight => true,
            Self::ScrollHotbarRight => false,
            Self::ScrollHotbarLeft => false,
            Self::RequestServerChunkInfo => false,
            Self::RequestServerPlayerData => false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum PlayerActions {
    BreakBlock(Vec3, EntityPos),
    PlaceBlock(Vec3, EntityPos),
    MoveForwards(Vec3),
    MoveBackwards(Vec3),
    MoveLeft(Vec3),
    MoveRight(Vec3),
    MoveUp,
    MoveDown,
    ScrollHotbarRight,
    ScrollHotbarLeft,
}
