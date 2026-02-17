use std::{fs, path::PathBuf};

use engine_core::paths::DATA_DIR;
use rustc_hash::FxHashMap;

use crate::{
    block_properties::BlockProperties,
    manifest::{BlockDefinition, BlockManifest, TextureValue},
};

#[derive(Default)]
pub struct BlockRegistry {
    properties: Vec<BlockProperties>,
    name_to_id: FxHashMap<String, u16>,
    id_to_name: Vec<String>,
}

impl BlockRegistry {
    pub fn init(include_assets: bool) -> (BlockRegistry, Vec<PathBuf>, Vec<u32>) {
        let mut registry = BlockRegistry::default();
        let mut texture_queue = Vec::new();
        let mut texture_to_id = std::collections::HashMap::new();
        let mut mapping_table = Vec::new();

        // breathe air
        mapping_table.extend_from_slice(&[0; 6]);

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
                            path.clone(),
                            path.clone(),
                            path.clone(),
                            path.clone(),
                            path.clone(),
                            path.clone(),
                        ],
                        TextureValue::Complex(texture_config) => texture_config.resolve_faces(),
                    };

                    for tex_name in face_textures {
                        let full_tex_path = ns.1.join("textures/blocks").join(tex_name);

                        let atlas_index = *texture_to_id
                            .entry(full_tex_path.clone())
                            .or_insert_with(|| {
                                let idx = texture_queue.len() as u32;
                                texture_queue.push(full_tex_path);
                                idx
                            });

                        mapping_table.push(atlas_index);
                    }
                }

                registry.register_block(full_id, block);
            }
        }

        println!("Mapping table: {:?}", mapping_table);

        (registry, texture_queue, mapping_table)
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
