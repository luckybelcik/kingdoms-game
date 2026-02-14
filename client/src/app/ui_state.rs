use engine_settings::client_config::{
    mesh_config::{MeshConfig, MeshFlags},
    push_constant_config::{PushConstantConfig, PushConstantFlags},
    render_config::{RenderConfig, RenderFlags},
};

#[derive(Default)]
pub struct UIState {
    pub show_ui: bool,
    pub popup_window: PopupWindow,
}

impl UIState {
    pub fn toggle_popup(&mut self, state: PopupWindow) {
        match &self.popup_window {
            PopupWindow::None => self.popup_window = state,
            PopupWindow::WorldSize(_) => self.popup_window = PopupWindow::None,
            PopupWindow::RenderConfig(_) => self.popup_window = PopupWindow::None,
        }
    }
}

#[derive(Default)]
pub enum PopupWindow {
    #[default]
    None,
    WorldSize(WorldSizePopupData),
    RenderConfig(RenderConfigData),
}

#[derive(Default)]
pub struct WorldSizePopupData {
    pub size: u32,
}

#[derive(Default)]
pub struct RenderConfigData {
    pub render_textures: bool,
    pub cull_chunk_faces: bool,
    pub greedy_meshing: bool,
}

impl RenderConfigData {
    pub fn new() -> Self {
        RenderConfigData {
            render_textures: PushConstantConfig::get(PushConstantFlags::RENDER_TEXTURES),
            cull_chunk_faces: RenderConfig::get(RenderFlags::CULL_FACES),
            greedy_meshing: MeshConfig::get(MeshFlags::GREEDY_MESH),
        }
    }
}
