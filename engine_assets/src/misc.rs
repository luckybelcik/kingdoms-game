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
