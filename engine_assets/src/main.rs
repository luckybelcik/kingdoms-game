use std::{error::Error, time::Instant};

use engine_assets::{AssetManager, misc::Timings};
use engine_core::paths::init_data_path;

fn main() -> Result<(), Box<dyn Error>> {
    let mut master_timings = Timings::default();
    let _ = init_data_path();

    let start_time = Instant::now();

    for _ in 0..50 {
        let asset_manager_timings = AssetManager::init(None, true).1;
        master_timings.add(&asset_manager_timings);
    }

    let time_elapsed = start_time.elapsed().as_millis();
    println!("Initialization time for 50x: {:?}ms", time_elapsed);
    println!("Avg time for 1x: {:?}ms", time_elapsed / 50);
    master_timings.print();
    Ok(())
}
