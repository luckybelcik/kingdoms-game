use std::{collections::HashMap, fs::File, io::Read};

use winit::keyboard::KeyCode;

use crate::client::{
    app::app_actions::AppKeybindableActions, client::client_actions::ClientKeybindableActions,
};

pub struct InputHandler {
    app_keybindings: HashMap<KeyCode, AppKeybindableActions>,
    client_keybindings: HashMap<KeyCode, ClientKeybindableActions>,
    key_hashmap: HashMap<KeyCode, ContainingMap>,
}

impl InputHandler {
    pub fn new() -> Self {
        let mut app_json_file = File::open("src/config/app_keys.json").unwrap();
        let mut client_json_file = File::open("src/config/client_keys.json").unwrap();

        let mut app_json_str = String::new();
        let mut client_json_str = String::new();

        let _ = File::read_to_string(&mut app_json_file, &mut app_json_str);
        let _ = File::read_to_string(&mut client_json_file, &mut client_json_str);

        let app_keybindings: HashMap<KeyCode, AppKeybindableActions> =
            serde_json::from_str(&app_json_str).unwrap();
        let client_keybindings: HashMap<KeyCode, ClientKeybindableActions> =
            serde_json::from_str(&client_json_str).unwrap();

        let mut key_hashmap = HashMap::new();

        for key in app_keybindings.keys() {
            if key_hashmap.insert(key.clone(), ContainingMap::App) != None {
                panic!(
                    "Duplicate keybindings fount. Also, if you see this, tell the programmer to fix this panic. It shouldn't happen. lol."
                );
            }
        }

        for key in client_keybindings.keys() {
            if key_hashmap.insert(key.clone(), ContainingMap::Client) != None {
                panic!(
                    "Duplicate keybindings fount. Also, if you see this, tell the programmer to fix this panic. It shouldn't happen. lol."
                )
            }
        }

        InputHandler {
            app_keybindings,
            client_keybindings,
            key_hashmap,
        }
    }

    pub fn handle_input(&self, input: &KeyCode) -> ActionOption {
        if let Some(containing_map) = self.key_hashmap.get(input) {
            match containing_map {
                ContainingMap::App => {
                    if let Some(action) = self.app_keybindings.get(input) {
                        return ActionOption::App(action.clone());
                    }
                }
                ContainingMap::Client => {
                    if let Some(action) = self.client_keybindings.get(input) {
                        return ActionOption::Client(action.clone());
                    }
                }
            }
        }

        return ActionOption::None;
    }
}

#[derive(PartialEq)]
enum ContainingMap {
    App,
    Client,
}

pub enum ActionOption {
    App(AppKeybindableActions),
    Client(ClientKeybindableActions),
    None,
}

impl ActionOption {
    pub fn is_single_press(&self) -> bool {
        match self {
            ActionOption::App(action) => action.is_single_press(),
            ActionOption::Client(action) => action.is_single_press(),
            ActionOption::None => false,
        }
    }

    pub fn is_holdable(&self) -> bool {
        match self {
            ActionOption::App(action) => action.is_holdable(),
            ActionOption::Client(action) => action.is_holdable(),
            ActionOption::None => false,
        }
    }
}
