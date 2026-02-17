use engine_core::chunk_pos::ChunkPos;

use crate::chunk_draw_call_info::ChunkDrawCallInfo;

pub mod block_data;
pub mod block_data_render;
pub mod chunk_draw_call_info;
pub mod constants;
pub mod gpu;
pub mod indirect;
pub mod per_draw_data;
pub mod push_constants;
pub mod render_results;
pub mod renderer;
pub mod scene;
pub mod texture_manager;
pub mod vertex;

pub struct ChunkDrawCommand {
    pub chunk_pos: ChunkPos,
    pub draw_call_info: Vec<ChunkDrawCallInfo>,
}
