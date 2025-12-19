use std::{collections::VecDeque, time::Instant};

use crate::shared::coordinate_systems::entity_pos::EntityPos;

#[derive(Default)]
pub struct AppInfo {
    pub last_render_time: Option<Instant>,
    pub last_size: (u32, u32),
    pub chunk_updates: u64,
    pub chunk_count: u64,
    pub total_chunk_vram: u64,
    pub avg_chunk_vram: u64,
    pub camera_pos: EntityPos,
    pub camera_rot: nalgebra_glm::Vec3,
    pub delta_history: VecDeque<u16>,
    pub avg_fps_history: VecDeque<u16>,
    pub accumulator: f64,
    pub tick: u128,
}
