#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AppRenderConfig {
    pub push_constant_data: u32,
    pub bool_data: u32,
    pub meshing_data: u32,
}

// push
pub const RENDER_TEXTURES_BIT: u32 = 1 << 0;

// bool
pub const CULL_CHUNK_FACES_BIT: u32 = 1 << 0;
pub const USE_LINE_RENDERING_BIT: u32 = 1 << 1;

// mesh
pub const GREEDY_MESHING_BIT: u32 = 1 << 0;

impl AppRenderConfig {
    #[inline]
    fn _set(data: &mut u32, mask: u32, value: bool) {
        *data = if value {
            *data | mask
        } else {
            *data & !mask
        };
    }

    #[inline]
    fn _get(data: u32, mask: u32) -> bool {
        (data & mask) != 0
    }


    // push
    pub fn set_render_textures_bit(&mut self, render_textures: bool) {
        Self::_set(&mut self.push_constant_data, RENDER_TEXTURES_BIT, render_textures);
    }

    pub fn get_render_textures_bit(&self) -> bool {
        Self::_get(self.push_constant_data, RENDER_TEXTURES_BIT)
    }

    pub fn toggle_render_textures_bit(&mut self) {
        self.push_constant_data ^= RENDER_TEXTURES_BIT;
    }

    // bool
    pub fn set_cull_chunk_faces_bit(&mut self, cull_chunk_faces: bool) {
        Self::_set(&mut self.bool_data, CULL_CHUNK_FACES_BIT, cull_chunk_faces);
    }

    pub fn get_cull_chunk_faces_bit(&self) -> bool {
        Self::_get(self.bool_data, CULL_CHUNK_FACES_BIT)
    }

    pub fn toggle_cull_chunk_faces_bit(&mut self) {
        self.bool_data ^= CULL_CHUNK_FACES_BIT;
    }

    pub fn set_use_line_rendering_bit(&mut self, use_line_rendering: bool) {
        Self::_set(&mut self.bool_data, USE_LINE_RENDERING_BIT, use_line_rendering);
    }

    pub fn get_use_line_rendering_bit(&self) -> bool {
        Self::_get(self.bool_data, USE_LINE_RENDERING_BIT)
    }

    pub fn toggle_use_line_rendering_bit(&mut self) {
        self.bool_data ^= USE_LINE_RENDERING_BIT;
    }

    // mesh
    pub fn set_greedy_meshing_bit(&mut self, greedy_meshing: bool) {
        Self::_set(&mut self.meshing_data, GREEDY_MESHING_BIT, greedy_meshing);
    }

    pub fn get_greedy_meshing_bit(&self) -> bool {
        Self::_get(self.meshing_data, GREEDY_MESHING_BIT)
    }

    pub fn toggle_greedy_meshing_bit(&mut self) {
        self.meshing_data ^= GREEDY_MESHING_BIT;
    }
}

impl Default for AppRenderConfig {
    fn default() -> Self {
        Self {
            push_constant_data: 1,
            bool_data: 0b01,
            meshing_data: 1,
        }
    }
}
