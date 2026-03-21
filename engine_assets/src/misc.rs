use std::path::PathBuf;

use image::DynamicImage;

pub struct AssetSlopConfig {
    pub block_padding: u32,
    pub mask_padding: u32,
    pub colormap_padding: u32,
}

impl Default for AssetSlopConfig {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        return Self {
            block_padding: 32,
            mask_padding: 16,
            colormap_padding: 4,
        };

        #[cfg(not(debug_assertions))]
        return Self {
            block_padding: 0,
            mask_padding: 0,
            colormap_padding: 0,
        };
    }
}

pub struct TextureUpdate {
    pub layer_index: u32,
    pub data: DynamicImage,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct MaskRecipe {
    pub paths: [Option<PathBuf>; 3],
}

#[derive(Default)]
pub struct AssetManagerMemory {
    pub active_projects: usize,

    pub active_block_textures: usize,
    pub active_colormap_masks_textures: usize,
    pub active_colormap_textures: usize,

    pub block_registry: usize,
    pub colormap_registry: usize,

    pub block_path_to_layer: usize,
    pub colormap_path_to_layer: usize,
    pub mask_dependencies: usize,
    pub active_mask_recipes: usize,

    pub block_upload_queue: usize,
    pub mask_upload_queue: usize,
    pub colormap_upload_queue: usize,

    pub texture_mapping_table: usize,
    pub metadata_table: usize,
    pub texture_variant_mapping_table: usize,
    pub colormap_mask_variant_mapping_table: usize,

    pub block_allocator: usize,
    pub mask_allocator: usize,
    pub colormap_allocator: usize,

    pub total: usize,
}

impl AssetManagerMemory {
    pub fn resolve_total(&mut self) {
        self.total = self.active_projects
            + self.active_block_textures
            + self.active_colormap_masks_textures
            + self.active_colormap_textures
            + self.block_registry
            + self.colormap_registry
            + self.block_path_to_layer
            + self.colormap_path_to_layer
            + self.mask_dependencies
            + self.active_mask_recipes
            + self.block_upload_queue
            + self.mask_upload_queue
            + self.colormap_upload_queue
            + self.texture_mapping_table
            + self.metadata_table
            + self.texture_variant_mapping_table
            + self.colormap_mask_variant_mapping_table
            + self.block_allocator
            + self.mask_allocator
            + self.colormap_allocator;
    }
}

#[derive(Default, Clone)]
pub struct Timings {
    pub project_finding: u128,
    pub block_registry_init: u128,
    pub image_loading: u128,
    pub allocator_setup: u128,
    pub watcher_setup: u128,
    pub total: u128,
}

impl Timings {
    pub fn add(&mut self, other: &Timings) {
        self.project_finding += other.project_finding;
        self.block_registry_init += other.block_registry_init;
        self.image_loading += other.image_loading;
        self.allocator_setup += other.allocator_setup;
        self.watcher_setup += other.watcher_setup;
        self.total += other.total;
    }

    pub fn print(&self) {
        let total_time = self.project_finding
            + self.block_registry_init
            + self.image_loading
            + self.allocator_setup
            + self.watcher_setup;
        println!(
            "Project Finding: {:?}ns ({:.1}%)",
            self.project_finding,
            self.project_finding as f64 / total_time as f64 * 100.0
        );
        println!(
            "Block Registry Init: {:?}ns ({:.1}%)",
            self.block_registry_init,
            self.block_registry_init as f64 / total_time as f64 * 100.0
        );
        println!(
            "Image Loading: {:?}ns ({:.1}%)",
            self.image_loading,
            self.image_loading as f64 / total_time as f64 * 100.0
        );
        println!(
            "Allocator Setup: {:?}ns ({:.1}%)",
            self.allocator_setup,
            self.allocator_setup as f64 / total_time as f64 * 100.0
        );
        println!(
            "Watcher Setup: {:?}ns ({:.1}%)",
            self.watcher_setup,
            self.watcher_setup as f64 / total_time as f64 * 100.0
        );
    }
}

pub enum PendingUpdate {
    MainShaderUpdate(String),
}
