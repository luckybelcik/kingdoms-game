#[derive(Default)]
pub struct RenderResults {
    pub triangles_rendered: u32,
    pub chunk_count: u32,
    pub draw_calls: u32,
}