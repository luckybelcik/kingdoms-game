use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use engine_core::paths::DATA_DIR;
use image::GrayImage;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    block_properties::BlockProperties,
    colormap_registry::ColormapRegistry,
    manifest::{BlockDefinition, BlockManifest, ColormapConfig, FaceConfig, TextureValue},
    rendering::{TextureMetadata, pack_colormap_ids, pack_sources},
};

#[derive(Default)]
pub struct BlockRegistry {
    properties: Vec<BlockProperties>,
    name_to_id: FxHashMap<String, u16>,
    id_to_name: Vec<String>,
}

impl BlockRegistry {
    pub fn init(
        include_assets: bool,
    ) -> (
        BlockRegistry,
        Vec<PathBuf>,
        Vec<GrayImage>,
        BTreeSet<PathBuf>,
        Vec<u32>,
        Vec<TextureMetadata>,
        Option<ColormapRegistry>,
    ) {
        let mut registry = BlockRegistry::default();
        let mut block_texture_queue = Vec::new();
        let mut block_colormap_mask_texture_queue = Vec::new();
        let mut colormap_queue = BTreeSet::default();
        let mut texture_to_id = FxHashMap::default();
        let mut mask_to_id: FxHashMap<Vec<u8>, i32> = FxHashMap::default();
        // facedir + blockid -> textureid
        let mut texture_mapping_table = Vec::new();
        // textureid -> texture metadata
        let mut metadata_mapping_table = Vec::new();
        let mut colormap_registry = ColormapRegistry::default(); // technically obsolete if not including assets but whatever

        // breathe air
        texture_mapping_table.extend_from_slice(&[0; 6]);

        let namespaces = discover_namespaces(DATA_DIR.get().cloned().unwrap());

        for ns in namespaces {
            let toml_path = ns.1.join("blocks.toml");
            if !toml_path.exists() {
                continue;
            }

            let file_content =
                std::fs::read_to_string(toml_path).expect("Failed to read blocks.toml");
            let manifest: BlockManifest = toml::from_str(&file_content).expect("Invalid TOML");

            for block in manifest.blocks {
                let full_id = if block.id.contains(':') {
                    block.id.clone()
                } else {
                    format!("{}:{}", ns.0, block.id)
                };

                if include_assets {
                    let face_textures = match &block.texture {
                        TextureValue::Simple(path) => [
                            FaceConfig::simple_from_path(path),
                            FaceConfig::simple_from_path(path),
                            FaceConfig::simple_from_path(path),
                            FaceConfig::simple_from_path(path),
                            FaceConfig::simple_from_path(path),
                            FaceConfig::simple_from_path(path),
                        ],
                        TextureValue::Complex(texture_config) => texture_config.resolve_faces(),
                    };

                    for face_config in face_textures {
                        let full_tex_path =
                            ns.1.join("textures/blocks").join(face_config.path.clone());

                        let atlas_index = *texture_to_id
                            .entry(full_tex_path.clone())
                            .or_insert_with(|| {
                                let idx = block_texture_queue.len() as u32;
                                block_texture_queue.push(full_tex_path.clone());
                                idx
                            });

                        texture_mapping_table.push(atlas_index);

                        let mask_atlas_idx = if face_config.colormap0.is_some()
                            || face_config.colormap1.is_some()
                            || face_config.colormap2.is_some()
                        {
                            let block_path = ns.1.join("textures/blocks");
                            let m0 = load_gray_or_empty(&face_config.colormap0, &block_path);
                            let m1 = load_gray_or_empty(&face_config.colormap1, &block_path);
                            let m2 = load_gray_or_empty(&face_config.colormap2, &block_path);

                            let packed_mask = blend_masks(&m0, &m1, &m2);
                            let raw_bytes = packed_mask.as_raw().clone();

                            *mask_to_id.entry(raw_bytes).or_insert_with(|| {
                                let idx = block_colormap_mask_texture_queue.len() as i32;
                                block_colormap_mask_texture_queue.push(packed_mask);
                                idx
                            })
                        } else {
                            -1
                        };

                        if let Some(c) = &face_config.colormap0 {
                            colormap_registry.get_or_register_asset(&c.map, &ns.1);
                            colormap_queue
                                .insert(ns.1.join("textures/colormaps").join(&c.map.clone()));
                        }
                        if let Some(c) = &face_config.colormap1 {
                            colormap_registry.get_or_register_asset(&c.map, &ns.1);
                        }
                        if let Some(c) = &face_config.colormap2 {
                            colormap_registry.get_or_register_asset(&c.map, &ns.1);
                        }

                        let metadata = TextureMetadata {
                            packed_colormap_ids: pack_colormap_ids(
                                &face_config,
                                &colormap_registry,
                                &ns.1,
                            ),
                            mask_atlas_id: mask_atlas_idx,
                            packed_source_ids: pack_sources(&face_config),
                            _padding: 0,
                        };

                        metadata_mapping_table.push(metadata);
                    }
                }

                registry.register_block(full_id, block);
            }
        }

        println!("Mapping table: {:?}", texture_mapping_table);

        if include_assets {
            (
                registry,
                block_texture_queue,
                block_colormap_mask_texture_queue,
                colormap_queue,
                texture_mapping_table,
                metadata_mapping_table,
                Some(colormap_registry),
            )
        } else {
            (
                registry,
                block_texture_queue,
                block_colormap_mask_texture_queue,
                colormap_queue,
                texture_mapping_table,
                metadata_mapping_table,
                None,
            )
        }
    }

    pub fn register_block(&mut self, id: String, block_definition: BlockDefinition) {
        let block_properties: BlockProperties = block_definition.into();

        println!("Registered block: {}", id);

        self.properties.push(block_properties);
        self.id_to_name.push(id.clone());
        self.name_to_id.insert(id, self.properties.len() as u16 - 1);
    }

    pub fn get_all_blocks(&self) -> Vec<(&String, &u16)> {
        self.name_to_id.iter().collect()
    }

    pub fn get_block(&self, name_id: &String) -> Option<&u16> {
        self.name_to_id.get(name_id)
    }
}

fn discover_namespaces(data_root: PathBuf) -> Vec<(String, PathBuf)> {
    let mut namespaces = Vec::new();

    if let Ok(entries) = fs::read_dir(data_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    namespaces.push((name.to_string(), path));
                }
            }
        }
    }
    namespaces
}

fn load_gray_or_empty(maybe_config: &Option<ColormapConfig>, base_dir: &Path) -> Option<GrayImage> {
    match maybe_config {
        Some(config) => {
            let full_path = base_dir.join(&config.mask);
            match image::open(&full_path) {
                Ok(img) => Some(img.to_luma8()),
                Err(_) => {
                    panic!("Warning: Could not load mask {:?}, using empty.", full_path);
                }
            }
        }
        None => None,
    }
}

// quantize the 3 masks to be 3 bits 3 bits 2 bits
pub fn blend_masks(
    m0: &Option<GrayImage>,
    m1: &Option<GrayImage>,
    m2: &Option<GrayImage>,
) -> GrayImage {
    let (width, height) = m0
        .as_ref()
        .or(m1.as_ref())
        .or(m2.as_ref())
        .map(|img| img.dimensions())
        .unwrap_or((16, 16));

    let mut out = GrayImage::new(width, height);

    let s0 = m0.as_ref().map(|img| img.as_raw());
    let s1 = m1.as_ref().map(|img| img.as_raw());
    let s2 = m2.as_ref().map(|img| img.as_raw());

    out.par_iter_mut().enumerate().for_each(|(i, pixel)| {
        let val0 = s0.map(|s| s[i] >> 5).unwrap_or(0);
        let val1 = s1.map(|s| (s[i] >> 5) << 3).unwrap_or(0);
        let val2 = s2.map(|s| (s[i] >> 6) << 6).unwrap_or(0);

        *pixel = val0 | val1 | val2;
    });

    out
}
