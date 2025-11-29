#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AppRenderConfig {
    pub data: u32,
}

pub const RENDER_TEXTURES_BIT: u32 = 1 << 0;

impl AppRenderConfig {
    pub fn set_render_textures_bit(&mut self, render_textures: bool) {
        self.data = if render_textures {
            self.data | RENDER_TEXTURES_BIT
        } else {
            self.data & !RENDER_TEXTURES_BIT
        };
    }

    pub fn get_render_textures_bit(&self) -> bool {
        (self.data & RENDER_TEXTURES_BIT) != 0
    }

    pub fn toggle_render_textures_bit(&mut self) {
        self.set_render_textures_bit(!self.get_render_textures_bit());
    }
}

impl Default for AppRenderConfig {
    fn default() -> Self {
        Self {
            data: 1,
        }
    }
}