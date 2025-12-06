#[derive(Default)]
pub struct RenderResults {
    pub triangles_rendered: u32,
    pub chunk_count: u32,
    pub draw_calls: u32,
    pub allocated_blocks: u32,
    pub total_space: u64,
    pub free_space: u64,
    pub total_chunk_vram: u64,
    pub avg_chunk_vram: u64,
}
