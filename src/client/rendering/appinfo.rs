use std::{collections::VecDeque, time::Instant};

#[derive(Default)]
pub struct AppInfo {
    pub(crate) last_render_time: Option<Instant>,
    pub(crate) last_size: (u32, u32),
    pub(crate) chunk_updates: u64,
    pub(crate) chunk_count: u64,
    pub(crate) total_chunk_vram: u64,
    pub(crate) avg_chunk_vram: u64,
    pub(crate) camera_pos: nalgebra_glm::Vec3,
    pub(crate) camera_rot: nalgebra_glm::Vec3,
    pub(crate) delta_history: VecDeque<u16>,
    pub(crate) avg_fps_history: VecDeque<u16>,
    pub(crate) tick: u128,
}