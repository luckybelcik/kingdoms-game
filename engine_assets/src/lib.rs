#![feature(iter_collect_into)]

use std::path::PathBuf;

use image::{DynamicImage, GenericImage, GenericImageView, GrayImage};

use crate::{
    block_registry::BlockRegistry, colormap_registry::ColormapRegistry, rendering::TextureMetadata,
};

pub mod block_properties;
pub mod block_registry;
pub mod colormap_registry;
pub mod manifest;
pub mod rendering;

#[derive(Default)]
pub struct AssetManager {
    pub block_registry: BlockRegistry,
    pub colormap_registry: ColormapRegistry,
    pub block_atlas: DynamicImage,
    pub block_colormap_mask_atlas: GrayImage,
    pub colormap_textures: Vec<DynamicImage>,
    pub texture_mapping_table: Vec<u32>,
    pub metadata_table: Vec<TextureMetadata>,
}

impl AssetManager {
    pub fn init() -> AssetManager {
        let (
            block_registry,
            block_texture_paths,
            block_colormap_masks,
            colormap_texture_paths,
            texture_mapping_table,
            metadata_table,
            colormap_registry,
        ) = BlockRegistry::init(true);

        let block_atlas = create_texture_atlas(&block_texture_paths);
        let block_colormap_mask_atlas =
            create_texture_atlas_from_gray_images(&block_colormap_masks);

        let mut colormap_textures = Vec::new();
        for path in colormap_texture_paths {
            let img = image::open(path).expect("Failed to load colormap texture");
            colormap_textures.push(img);
        }

        AssetManager {
            block_registry,
            colormap_registry: colormap_registry.unwrap(),
            block_atlas,
            block_colormap_mask_atlas,
            colormap_textures,
            texture_mapping_table,
            metadata_table,
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

pub fn create_texture_atlas_from_gray_images(images: &[GrayImage]) -> GrayImage {
    let texture_count = images.len() as u32;

    let side_in_blocks = (texture_count as f32).sqrt().ceil() as u32;
    let pixel_side = side_in_blocks * 16;

    let mut atlas = GrayImage::new(pixel_side, pixel_side);

    for (i, img) in images.iter().enumerate() {
        if img.dimensions() != (16, 16) {
            panic!("Gray texture is not 16x16! It's {:?} x3", img.dimensions());
        }

        let grid_x = (i as u32 % side_in_blocks) * 16;
        let grid_y = (i as u32 / side_in_blocks) * 16;

        atlas
            .copy_from(img, grid_x, grid_y)
            .expect("Failed to copy texture into atlas");
    }

    atlas
}
