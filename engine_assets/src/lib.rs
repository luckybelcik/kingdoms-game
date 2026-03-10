#![feature(iter_collect_into)]

use image::DynamicImage;

use crate::{
    block_registry::BlockRegistry, colormap_registry::ColormapRegistry, projects::Project,
    rendering::TextureMetadata,
};

pub mod block_properties;
pub mod block_registry;
pub mod colormap_registry;
pub mod manifest;
pub mod projects;
pub mod rendering;

#[derive(Default)]
pub struct AssetManager {
    pub block_registry: BlockRegistry,
    pub colormap_registry: ColormapRegistry,
    pub block_textures: Vec<DynamicImage>,
    pub block_colormap_mask_array: Vec<DynamicImage>,
    pub colormap_textures: Vec<DynamicImage>,
    pub texture_mapping_table: Vec<u32>,
    pub metadata_table: Vec<TextureMetadata>,
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

        let mut block_textures = Vec::new();
        for path in block_texture_paths {
            let img = image::open(path).expect("Failed to load block texture");
            block_textures.push(img);
        }

        let mut block_colormap_mask_array = Vec::new();
        for mask in block_colormap_masks {
            block_colormap_mask_array.push(DynamicImage::ImageLuma8(mask));
        }

        let mut colormap_textures = Vec::new();
        for path in colormap_texture_paths {
            let img = image::open(path).expect("Failed to load colormap texture");
            colormap_textures.push(img);
        }

        AssetManager {
            block_registry,
            colormap_registry: colormap_registry.unwrap(),
            block_textures,
            block_colormap_mask_array,
            colormap_textures,
            texture_mapping_table,
            metadata_table,
        }
    }
}
