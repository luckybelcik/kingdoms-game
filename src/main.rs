#![feature(int_roundings)]

use std::{fs, path::PathBuf, sync::OnceLock};

use directories::ProjectDirs;
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::{
    client::{
        app::app::App,
        connection_details::{ClientConnectionType, LocalConnectionDetails},
    },
    server::server::Server,
    shared::communication::player_id::PlayerId,
};

pub mod client;
pub mod server;
pub mod shared;

pub static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        #[cfg(target_os = "linux")]
        let event_loop = winit::event_loop::EventLoop::builder().with_x11().build()?;

        #[cfg(not(target_os = "linux"))]
        let event_loop = winit::event_loop::EventLoop::builder().build()?;

        init_data_path().expect("Failed to initialize data path:");

        let (server_sender, server_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let (client_sender, client_receiver) = std::sync::mpsc::channel::<Vec<u8>>();

        let player_id = PlayerId::new();
        let player_id_clone = player_id.clone();

        std::thread::Builder::new()
            .name("MainServerThread".to_string())
            .spawn(move || {
                let mut server = Server::new();

                server.add_local_player(player_id_clone, server_sender, client_receiver);

                server.run_tick_loop();
            })
            .unwrap();

        println!("Server set up successfully! Jump jump jump!");

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let local_connection_details = LocalConnectionDetails {
            server_packet_receiver: server_receiver,
            client_packet_sender: client_sender,
        };
        let connection_type = ClientConnectionType::Local(local_connection_details);
        let mut app = App::new(player_id, connection_type);
        println!("App created successfully!");
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

fn init_data_path() -> Result<(), std::io::Error> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "stargrazer-games", "kingdomsgame") {
        let config_dir = proj_dirs.config_dir();
        let data_dir = proj_dirs.data_dir();

        // create dirs if dont exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir).unwrap();
        }

        if !data_dir.exists() {
            fs::create_dir_all(data_dir).unwrap();
        }

        CONFIG_DIR.get_or_init(|| config_dir.to_path_buf());
        DATA_DIR.get_or_init(|| data_dir.to_path_buf());

        println!("Config dir: {}", config_dir.to_str().unwrap());
        println!("Data dir: {}", data_dir.to_str().unwrap());

        let app_key_config = config_dir.join("app_keys.json");
        let client_key_config = config_dir.join("client_keys.json");

        // create files if dont exist
        if !app_key_config.exists() {
            let default_data = r#"{
                    "Escape": "ExitApp",
                    "KeyP": "ToggleTextureRendering",
                    "KeyL": "ToggleLineRendering",
                    "F3": "ToggleDebugUI"
                }"#;
            fs::write(&app_key_config, default_data)?;
        }

        if !client_key_config.exists() {
            let default_data = r#"{
                    "Comma": "BreakBlock",
                    "Period": "PlaceBlock",
                    "KeyW": "MoveForwards",
                    "KeyS": "MoveBackwards",
                    "KeyA": "MoveLeft",
                    "KeyD": "MoveRight",
                    "Space": "MoveUp",
                    "ShiftLeft": "MoveDown",
                    "ArrowUp": "RotateUp",
                    "ArrowDown": "RotateDown",
                    "ArrowLeft": "RotateLeft",
                    "ArrowRight": "RotateRight",
                    "Equal": "ScrollHotbarRight",
                    "Minus": "ScrollHotbarLeft",
                    "KeyI": "RequestServerPlayerData",
                    "KeyU": "RequestServerChunkInfo"
                }"#;
            fs::write(&client_key_config, default_data).unwrap();
        }

        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not determine home directory",
    ))
}
