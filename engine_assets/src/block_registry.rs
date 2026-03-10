use std::{collections::BTreeSet, path::PathBuf};

use rustc_hash::FxHashMap;

use crate::{
    block_properties::BlockProperties,
    colormap_registry::ColormapRegistry,
    manifest::{BlockDefinition, BlockManifest, FaceConfig, TextureValue},
    misc::MaskRecipe,
    projects::Project,
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
        projects: &[Project],
        include_assets: bool,
    ) -> (
        Vec<Project>,
        BlockRegistry,
        Vec<PathBuf>,
        Vec<MaskRecipe>,
        BTreeSet<PathBuf>,
        Vec<u32>,
        Vec<TextureMetadata>,
        Option<ColormapRegistry>,
    ) {
        let mut loaded_projects = Vec::new();
        let mut registry = BlockRegistry::default();
        let mut block_texture_queue = Vec::new();
        let mut mask_recipes_queue = Vec::new();
        let mut colormap_queue = BTreeSet::default();
        let mut texture_to_id = FxHashMap::default();
        let mut mask_to_id: FxHashMap<MaskRecipe, i32> = FxHashMap::default();
        // facedir + blockid -> textureid
        let mut texture_mapping_table = Vec::new();
        // textureid -> texture metadata
        let mut metadata_mapping_table = Vec::new();
        let mut colormap_registry = ColormapRegistry::default(); // technically obsolete if not including assets but whatever

        // breathe air
        // basically we add an entry for air, i know there was a good reason for it but i forgot xdd
        texture_mapping_table.extend_from_slice(&[0; 6]);

        for project in projects {
            let toml_path = project.path.join("blocks.toml");
            if !toml_path.exists() {
                continue;
            } else {
                loaded_projects.push(project.clone());
            }

            let file_content =
                std::fs::read_to_string(toml_path).expect("Failed to read blocks.toml");
            let manifest: BlockManifest = toml::from_str(&file_content).expect("Invalid TOML");

            for block in manifest.blocks {
                let full_id = if block.id.contains(':') {
                    block.id.clone()
                } else {
                    format!("{}:{}", project.name, block.id)
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
                        let full_tex_path = project
                            .path
                            .join("textures/blocks")
                            .join(face_config.path.clone());

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
                            let block_path = project.path.join("textures/blocks");

                            let recipe = MaskRecipe {
                                paths: [
                                    face_config
                                        .colormap0
                                        .as_ref()
                                        .map(|c| block_path.join(&c.mask)),
                                    face_config
                                        .colormap1
                                        .as_ref()
                                        .map(|c| block_path.join(&c.mask)),
                                    face_config
                                        .colormap2
                                        .as_ref()
                                        .map(|c| block_path.join(&c.mask)),
                                ],
                            };

                            *mask_to_id.entry(recipe.clone()).or_insert_with(|| {
                                let idx = mask_recipes_queue.len() as i32;
                                mask_recipes_queue.push(recipe);
                                idx
                            })
                        } else {
                            -1
                        };

                        if let Some(c) = &face_config.colormap0 {
                            colormap_registry.get_or_register_asset(&c.map, &project.path);
                            colormap_queue.insert(
                                project.path.join("textures/colormaps").join(&c.map.clone()),
                            );
                        }
                        if let Some(c) = &face_config.colormap1 {
                            colormap_registry.get_or_register_asset(&c.map, &project.path);
                        }
                        if let Some(c) = &face_config.colormap2 {
                            colormap_registry.get_or_register_asset(&c.map, &project.path);
                        }

                        let metadata = TextureMetadata {
                            packed_colormap_ids: pack_colormap_ids(
                                &face_config,
                                &colormap_registry,
                                &project.path,
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
                loaded_projects,
                registry,
                block_texture_queue,
                mask_recipes_queue,
                colormap_queue,
                texture_mapping_table,
                metadata_mapping_table,
                Some(colormap_registry),
            )
        } else {
            (
                loaded_projects,
                registry,
                block_texture_queue,
                mask_recipes_queue,
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
