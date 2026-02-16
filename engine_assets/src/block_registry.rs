use std::{fs, path::PathBuf};

use engine_core::paths::DATA_DIR;
use rustc_hash::FxHashMap;

use crate::{
    block_properties::BlockProperties,
    manifest::{BlockDefinition, BlockManifest},
};

#[derive(Default)]
pub struct BlockRegistry {
    properties: Vec<BlockProperties>,
    name_to_id: FxHashMap<String, u16>,
    id_to_name: Vec<String>,
}

impl BlockRegistry {
    pub fn init_with_textures() -> (BlockRegistry, Vec<PathBuf>) {
        let mut registry = BlockRegistry::default();
        let mut texture_queue = Vec::new();

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

                let tex_path = ns.1.join("textures/blocks").join(&block.texture_name);
                texture_queue.push(tex_path);

                registry.register_block(full_id, block);
            }
        }

        (registry, texture_queue)
    }

    pub fn register_block(&mut self, id: String, block_definition: BlockDefinition) {
        let block_properties: BlockProperties = block_definition.into();

        println!("Registered block: {}", id);

        self.properties.push(block_properties);
        self.id_to_name.push(id.clone());
        self.name_to_id.insert(id, self.properties.len() as u16 - 1);
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
