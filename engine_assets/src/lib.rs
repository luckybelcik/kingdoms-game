#![feature(iter_collect_into)]

use image::DynamicImage;

use crate::{
    block_registry::BlockRegistry,
    colormap_registry::ColormapRegistry,
    layer_allocator::LayerAllocator,
    misc::{AssetSlopConfig, TextureUpdate},
    projects::Project,
    rendering::TextureMetadata,
};

pub mod block_properties;
pub mod block_registry;
pub mod colormap_registry;
pub mod layer_allocator;
pub mod manifest;
pub mod misc;
pub mod projects;
pub mod rendering;

pub struct AssetManager {
    pub block_registry: BlockRegistry,
    pub colormap_registry: ColormapRegistry,

    pub block_upload_queue: Vec<TextureUpdate>,
    pub mask_upload_queue: Vec<TextureUpdate>,
    pub colormap_upload_queue: Vec<TextureUpdate>,

    pub texture_mapping_table: Vec<u32>,
    pub metadata_table: Vec<TextureMetadata>,

    pub block_allocator: LayerAllocator,
    pub mask_allocator: LayerAllocator,
    pub colormap_allocator: LayerAllocator,
}

impl AssetManager {
    pub fn init(load_projects: Option<Vec<String>>, load_native_by_default: bool) -> AssetManager {
        let mut projects_to_load = Vec::new();

        if load_native_by_default {
            if let Some(native) = Project::find("native") {
                projects_to_load.push(native);
            } else {
                panic!("Critical Error: 'native' project not found!");
            }
        }

        if let Some(names) = load_projects {
            for name in names {
                if name == "native" && load_native_by_default {
                    continue;
                }
                if let Some(proj) = Project::find(&name) {
                    projects_to_load.push(proj);
                } else {
                    eprintln!("Warning: Requested project '{}' not found.", name);
                }
            }
        } else {
            // Load all available projects if no specific list provided
            let all_projects = Project::find_all();
            for proj in all_projects {
                if proj.name == "native" && load_native_by_default {
                    continue;
                }
                projects_to_load.push(proj);
            }
        }

        let (
            block_registry,
            block_texture_paths,
            block_colormap_masks,
            colormap_texture_paths,
            texture_mapping_table,
            metadata_table,
            colormap_registry,
        ) = BlockRegistry::init(&projects_to_load, true);

        let block_upload_queue: Vec<TextureUpdate> = block_texture_paths
            .into_iter()
            .enumerate()
            .map(|(i, p)| TextureUpdate {
                layer_index: i as u32,
                data: image::open(p).expect("Failed to load block texture"),
            })
            .collect();

        let mask_upload_queue: Vec<TextureUpdate> = block_colormap_masks
            .into_iter()
            .enumerate()
            .map(|(i, m)| TextureUpdate {
                layer_index: i as u32,
                data: DynamicImage::ImageLuma8(m),
            })
            .collect();

        let colormap_upload_queue: Vec<TextureUpdate> = colormap_texture_paths
            .into_iter()
            .enumerate()
            .map(|(i, p)| TextureUpdate {
                layer_index: i as u32,
                data: image::open(p).expect("Failed to load colormap texture"),
            })
            .collect();

        let config = AssetSlopConfig::default();

        let block_allocator =
            LayerAllocator::new(block_upload_queue.len() as u32, config.block_padding);
        let mask_allocator =
            LayerAllocator::new(mask_upload_queue.len() as u32, config.mask_padding);
        let colormap_allocator =
            LayerAllocator::new(colormap_upload_queue.len() as u32, config.colormap_padding);

        AssetManager {
            block_registry,
            colormap_registry: colormap_registry.unwrap(),

            block_upload_queue,
            mask_upload_queue,
            colormap_upload_queue,

            texture_mapping_table,
            metadata_table,

            block_allocator,
            mask_allocator,
            colormap_allocator,
        }
    }

    /// Clears the queues and shrinks them to fit.
    pub fn clear_queues(&mut self) {
        self.block_upload_queue.clear();
        self.mask_upload_queue.clear();
        self.colormap_upload_queue.clear();

        self.block_upload_queue.shrink_to_fit();
        self.mask_upload_queue.shrink_to_fit();
        self.colormap_upload_queue.shrink_to_fit();
    }
}
