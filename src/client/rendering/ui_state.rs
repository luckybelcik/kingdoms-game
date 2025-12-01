use crate::client::rendering::apprenderconfig::AppRenderConfig;

#[derive(Default)]
pub struct UIState {
    pub popup_window: PopupWindow
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
}

impl RenderConfigData {
    pub fn new(config: &AppRenderConfig) -> Self {
        RenderConfigData { render_textures: config.get_render_textures_bit(), cull_chunk_faces: config.get_cull_chunk_faces_bit() }
    }
}