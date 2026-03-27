use std::{error::Error, time::Instant};

use engine_assets::{AssetManager, misc::Timings};
use engine_core::paths::init_data_path;

fn main() -> Result<(), Box<dyn Error>> {
    let mut master_timings = Timings::default();
    let mut master_asset_manager = None;
    let _ = init_data_path();

    let start_time = Instant::now();

    for _ in 0..50 {
        let results = AssetManager::init(None, true, true).unwrap();
        master_timings.add(&results.1);
        master_asset_manager = Some(results.0);
    }

    let time_elapsed = start_time.elapsed().as_millis();
    println!("Initialization time for 50x: {:?}ms", time_elapsed);
    println!("Avg time for 1x: {:?}ms", time_elapsed / 50);
    println!();
    if let Some(asset_manager) = &master_asset_manager {
        let usage = asset_manager.estimate_memory_usage();
        println!("Memory usage:");
        println!("	active_projects: {}", usage.active_projects);
        println!(
            "	block_id_to_manifest_path: {}",
            usage.block_id_to_manifest_path
        );
        println!("	active_block_textures: {}", usage.active_block_textures);
        println!(
            "	active_colormap_masks_textures: {}",
            usage.active_colormap_masks_textures
        );
        println!(
            "	active_colormap_textures: {}",
            usage.active_colormap_textures
        );
        println!("	block_registry: {}", usage.block_registry);
        println!("	colormap_registry: {}", usage.colormap_registry);
        println!("	block_path_to_layer: {}", usage.block_path_to_layer);
        println!("	colormap_path_to_layer: {}", usage.colormap_path_to_layer);
        println!("	mask_dependencies: {}", usage.mask_dependencies);
        println!("	active_mask_recipes: {}", usage.active_mask_recipes);
        println!("	block_upload_queue: {}", usage.block_upload_queue);
        println!("	mask_upload_queue: {}", usage.mask_upload_queue);
        println!("	colormap_upload_queue: {}", usage.colormap_upload_queue);
        println!("	texture_mapping_table: {}", usage.texture_mapping_table);
        println!("	metadata_table: {}", usage.metadata_table);
        println!(
            "	texture_variant_mapping_table: {}",
            usage.texture_variant_mapping_table
        );
        println!(
            "	colormap_mask_variant_mapping_table: {}",
            usage.colormap_mask_variant_mapping_table
        );
        println!("	block_allocator: {}", usage.block_allocator);
        println!("	mask_allocator: {}", usage.mask_allocator);
        println!("	colormap_allocator: {}", usage.colormap_allocator);
        println!("	lasso: {}", usage.lasso);
        println!("	total: {}", usage.total);
    }
    println!();
    master_timings.print();
    println!("debug view");
    if let Some(asset_manager) = &master_asset_manager
        && false
    {
        println!("	active_projects: {:?}", asset_manager.active_projects);
        println!("	block_registry: {:?}", asset_manager.block_registry);
        println!("	colormap_registry: {:?}", asset_manager.colormap_registry);
        println!(
            "	block_path_to_layer: {:?}",
            asset_manager.block_path_to_layer
        );
        println!(
            "	colormap_path_to_layer: {:?}",
            asset_manager.colormap_path_to_layer
        );
        println!(
            "	colormap_mask_dependencies: {:?}",
            asset_manager.colormap_mask_dependencies
        );
        println!(
            "	active_mask_recipes: {:?}",
            asset_manager.active_mask_recipes
        );
        println!(
            "	texture_mapping_table: {:?}",
            asset_manager.texture_mapping_table
        );
        println!("	metadata_table: {:?}", asset_manager.metadata_table);
        println!(
            "	texture_variant_mapping_table: {:?}",
            asset_manager.texture_variant_mapping_table
        );
        println!(
            "	colormap_mask_variant_mapping_table: {:?}",
            asset_manager.colormap_mask_variant_mapping_table
        );
        println!("	block_allocator: {:?}", asset_manager.block_allocator);
        println!(
            "	colormap_mask_allocator: {:?}",
            asset_manager.colormap_mask_allocator
        );
        println!(
            "	colormap_allocator: {:?}",
            asset_manager.colormap_allocator
        );
        println!("	lasso: {:?}", asset_manager.interner);
    }

    Ok(())
}
