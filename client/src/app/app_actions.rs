use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppKeybindableActions {
    ExitApp,
    ToggleTextureRendering,
    ToggleLineRendering,
    ToggleDebugUI,
    ForceCrash,
    ReloadAssets,
}

impl AppKeybindableActions {
    pub fn is_single_press(&self) -> bool {
        match self {
            AppKeybindableActions::ExitApp => true,
            AppKeybindableActions::ToggleDebugUI => true,
            AppKeybindableActions::ToggleLineRendering => true,
            AppKeybindableActions::ToggleTextureRendering => true,
            AppKeybindableActions::ForceCrash => true,
            AppKeybindableActions::ReloadAssets => true,
        }
    }

    pub fn is_holdable(&self) -> bool {
        !self.is_single_press()
    }
}
