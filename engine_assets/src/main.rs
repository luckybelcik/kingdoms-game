use std::{error::Error, time::Instant};

use engine_assets::AssetManager;
use engine_core::paths::init_data_path;

fn main() -> Result<(), Box<dyn Error>> {
    let start_time = Instant::now();

    let _ = init_data_path();

    for _ in 0..50 {
        let asset_manager = AssetManager::init(None, true);
    }

    let time_elapsed = start_time.elapsed().as_millis();
    println!("Initialization time for 50x: {:?}ms", time_elapsed);
    println!("Avg time for 1x: {:?}ms", time_elapsed / 50);
    Ok(())
}
