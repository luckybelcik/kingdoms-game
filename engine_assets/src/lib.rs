use std::path::PathBuf;

use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer};

use crate::block_registry::BlockRegistry;

pub mod block_properties;
pub mod block_registry;
pub mod manifest;

#[derive(Default)]
pub struct AssetManager {
    pub block_registry: BlockRegistry,
    pub atlas: DynamicImage,
}

impl AssetManager {
    pub fn init() -> AssetManager {
        let (registry, texture_paths) = BlockRegistry::init_with_textures();

        let atlas = create_texture_atlas(&texture_paths);

        AssetManager {
            block_registry: registry,
            atlas,
        }
    }
}

pub fn create_texture_atlas(texture_paths: &[PathBuf]) -> DynamicImage {
    let block_count = texture_paths.len() as u32;

    let side_in_blocks = (block_count as f32).sqrt().ceil() as u32;
    let pixel_side = side_in_blocks * 16;

    let mut atlas = DynamicImage::new(pixel_side, pixel_side, image::ColorType::Rgba8);

    for (i, path) in texture_paths.iter().enumerate() {
        let img = image::open(path).expect("Failed to load block texture");

        if img.dimensions() != (16, 16) {
            panic!(
                "Texture {:?} is not 16x16! It's {:?} x3",
                path,
                img.dimensions()
            );
        }

        let grid_x = (i as u32 % side_in_blocks) * 16;
        let grid_y = (i as u32 / side_in_blocks) * 16;

        atlas
            .copy_from(&img, grid_x, grid_y)
            .expect("Failed to copy texture into atlas");
    }

    atlas
}
