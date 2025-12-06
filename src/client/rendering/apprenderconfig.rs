#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AppRenderConfig {
    pub push_constant_data: u32,
    pub bool_data: u32,
}

// push
pub const RENDER_TEXTURES_BIT: u32 = 1 << 0;

// bool
pub const CULL_CHUNK_FACES_BIT: u32 = 1 << 0;
pub const USE_LINE_RENDERING_BIT: u32 = 1 << 1;

impl AppRenderConfig {
    // push
    pub fn set_render_textures_bit(&mut self, render_textures: bool) {
        self.push_constant_data = if render_textures {
            self.push_constant_data | RENDER_TEXTURES_BIT
        } else {
            self.push_constant_data & !RENDER_TEXTURES_BIT
        };
    }

    pub fn get_render_textures_bit(&self) -> bool {
        (self.push_constant_data & RENDER_TEXTURES_BIT) != 0
    }

    pub fn toggle_render_textures_bit(&mut self) {
        self.set_render_textures_bit(!self.get_render_textures_bit());
    }

    // bool
    pub fn set_cull_chunk_faces_bit(&mut self, cull_chunk_faces: bool) {
        self.bool_data = if cull_chunk_faces {
            self.bool_data | CULL_CHUNK_FACES_BIT
        } else {
            self.bool_data & !CULL_CHUNK_FACES_BIT
        };
    }

    pub fn get_cull_chunk_faces_bit(&self) -> bool {
        (self.bool_data & CULL_CHUNK_FACES_BIT) != 0
    }

    pub fn toggle_cull_chunk_faces_bit(&mut self) {
        self.set_cull_chunk_faces_bit(!self.get_cull_chunk_faces_bit());
    }

    pub fn set_use_line_rendering_bit(&mut self, use_line_rendering: bool) {
        self.bool_data = if use_line_rendering {
            self.bool_data | USE_LINE_RENDERING_BIT
        } else {
            self.bool_data & !USE_LINE_RENDERING_BIT
        };
    }

    pub fn get_use_line_rendering_bit(&self) -> bool {
        (self.bool_data & USE_LINE_RENDERING_BIT) != 0
    }

    pub fn toggle_use_line_rendering_bit(&mut self) {
        self.set_use_line_rendering_bit(!self.get_use_line_rendering_bit());
    }
}

impl Default for AppRenderConfig {
    fn default() -> Self {
        Self {
            push_constant_data: 1,
            bool_data: 0b01,
        }
    }
}
