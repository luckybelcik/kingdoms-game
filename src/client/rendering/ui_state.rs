#[derive(Default)]
pub struct UIState {
    pub popup_window: PopupWindow
}

impl UIState {
    pub fn toggle_popup(&mut self, state: PopupWindow) {
        match &self.popup_window {
            PopupWindow::None => self.popup_window = state,
            PopupWindow::WorldSize(_) => self.popup_window = PopupWindow::None,
        }
    }
}

#[derive(Default)]
pub enum PopupWindow {
    #[default]
    None,
    WorldSize(WorldSizePopupData),
}

#[derive(Default)]
pub struct WorldSizePopupData {
    pub size: u32,
}