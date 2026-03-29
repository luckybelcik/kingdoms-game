use std::{fs, path::PathBuf, sync::OnceLock};

use directories::ProjectDirs;

pub static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init_data_path() -> Result<(), std::io::Error> {
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
            const DEFAULT_APP_KEYS: &[u8] = include_bytes!("../../config/app_keys.json");
            fs::write(&app_key_config, DEFAULT_APP_KEYS)?;
        }

        if !client_key_config.exists() {
            const DEFAULT_CLIENT_KEYS: &[u8] = include_bytes!("../../config/client_keys.json");
            fs::write(&client_key_config, DEFAULT_CLIENT_KEYS)?;
        }

        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not determine home directory",
    ))
}
