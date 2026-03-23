use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

use lasso::ThreadedRodeo;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use walkdir::WalkDir;

use crate::{
    block_properties::BlockProperties,
    colormap_registry::ColormapRegistry,
    engine_path::EnginePath,
    manifest::{BlockDefinition, BlockManifest, FaceConfigWithVariants, FacesOptions},
    misc::MaskRecipe,
    projects::Project,
    rendering::{TextureMetadata, pack_colormap_ids, pack_sources},
};

pub struct BlockRegistryContext {
    pub loaded_projects: Vec<Project>,
    pub block_registry: BlockRegistry,
    pub block_texture_paths: Vec<PathBuf>,
    pub block_colormap_masks: Vec<MaskRecipe>,
    pub colormap_texture_paths: BTreeSet<PathBuf>,
    pub texture_or_variant_mapping_table: Vec<u32>,
    pub metadata_table: Vec<TextureMetadata>,
    pub block_variant_mapping_table: Vec<u32>,
    pub colormap_mask_variant_mapping_table: Vec<u32>,
    pub colormap_registry: Option<ColormapRegistry>,
}

#[derive(Default, Debug)]
pub struct BlockRegistry {
    properties: Vec<BlockProperties>,
    name_to_id: FxHashMap<String, u16>,
    id_to_name: Vec<String>,
}

impl BlockRegistry {
    pub fn init(
        projects: Vec<Project>,
        include_assets: bool,
        interner: &Arc<ThreadedRodeo>,
    ) -> BlockRegistryContext {
        let parsed_projects: Vec<_> = projects
            .into_par_iter()
            .filter_map(|project| {
                let paths: Vec<PathBuf> = WalkDir::new(&project.path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().file_name().map_or(false, |name| {
                            name.to_string_lossy().ends_with("_blocks.toml")
                        })
                    })
                    .map(|e| e.into_path())
                    .collect();

                let manifests: Vec<BlockManifest> = paths
                    .into_par_iter()
                    .map(|path| {
                        let file_content =
                            std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());
                        toml::from_str(&file_content).unwrap()
                    })
                    .collect();

                Some((project, manifests))
            })
            .collect();

        let mut loaded_projects = Vec::new();
        let mut block_registry = BlockRegistry::default();
        let mut block_texture_paths = Vec::new();
        let mut block_colormap_masks = Vec::new();
        let mut colormap_texture_paths = BTreeSet::default();
        let mut texture_to_id = FxHashMap::default();
        let mut mask_to_id: FxHashMap<MaskRecipe, u32> = FxHashMap::default();
        // facedir + blockid -> textureid OR variant data
        let mut texture_or_variant_mapping_table = Vec::new();
        // textureid -> texture metadata
        let mut metadata_table = Vec::new();
        // variant data -> texture index to use
        let mut block_variant_mapping_table = Vec::new();
        // variant data -> colormap mask index to use
        let mut colormap_mask_variant_mapping_table = Vec::new();
        let mut colormap_registry = ColormapRegistry::default(); // technically obsolete if not including assets but whatever

        // breathe air
        // basically we add an entry for air, i know there was a good reason for it but i forgot xdd
        texture_or_variant_mapping_table.extend_from_slice(&[0; 6]);
        block_registry.register_block(
            "native:air".to_string(),
            BlockDefinition {
                id: "naive:air".to_string(),
                faces: FacesOptions::Unified("air".to_string()),
            },
        );

        for (project, manifests) in parsed_projects {
            for manifest in manifests {
                for block in manifest.blocks {
                    let full_id = if block.id.contains(':') {
                        block.id.clone()
                    } else {
                        format!("{}:{}", project.name, block.id)
                    };

                    if include_assets {
                        let face_configs = match &block.faces {
                            FacesOptions::Unified(path) => [
                                FaceConfigWithVariants::simple_from_path(path),
                                FaceConfigWithVariants::simple_from_path(path),
                                FaceConfigWithVariants::simple_from_path(path),
                                FaceConfigWithVariants::simple_from_path(path),
                                FaceConfigWithVariants::simple_from_path(path),
                                FaceConfigWithVariants::simple_from_path(path),
                            ],
                            FacesOptions::Unique(texture_config) => texture_config.resolve_faces(),
                        };

                        for face_config in face_configs {
                            let mut texture_ids = Vec::new();
                            let mut colormap_mask_ids = Vec::new();
                            let block_path = project.path.join("textures/blocks");

                            if let Some(c) = &face_config.colormap0 {
                                colormap_registry.get_or_register_asset(
                                    &c.map,
                                    &project.path,
                                    &interner,
                                );
                                colormap_texture_paths.insert(
                                    project.path.join("textures/colormaps").join(&c.map.clone()),
                                );
                            }
                            if let Some(c) = &face_config.colormap1 {
                                colormap_registry.get_or_register_asset(
                                    &c.map,
                                    &project.path,
                                    &interner,
                                );
                                colormap_texture_paths.insert(
                                    project.path.join("textures/colormaps").join(&c.map.clone()),
                                );
                            }
                            if let Some(c) = &face_config.colormap2 {
                                colormap_registry.get_or_register_asset(
                                    &c.map,
                                    &project.path,
                                    &interner,
                                );
                                colormap_texture_paths.insert(
                                    project.path.join("textures/colormaps").join(&c.map.clone()),
                                );
                            }

                            for face in &face_config.faces {
                                // part 1 - handle regular textures
                                let full_tex_path = block_path.join(face.texture.clone());

                                // get texture id
                                let texture_id = *texture_to_id
                                    .entry(full_tex_path.clone())
                                    .or_insert_with(|| {
                                        let idx = block_texture_paths.len() as u32;
                                        block_texture_paths.push(full_tex_path.clone());
                                        idx
                                    });

                                texture_ids.push(texture_id);

                                // part 2 - handle colormap masks
                                // basically the if logic is if any paired colormap mask and colormap definitions exist
                                let colormap_mask_id = if (face.colormap0_mask.is_some()
                                    && face_config.colormap0.is_some())
                                    || (face.colormap1_mask.is_some()
                                        && face_config.colormap1.is_some())
                                    || (face.colormap2_mask.is_some()
                                        && face_config.colormap2.is_some())
                                {
                                    let recipe = MaskRecipe {
                                        paths: [
                                            face.colormap0_mask.as_ref().map(|c| {
                                                EnginePath::from_path(
                                                    &block_path.join(&c),
                                                    interner,
                                                )
                                            }),
                                            face.colormap1_mask.as_ref().map(|c| {
                                                EnginePath::from_path(
                                                    &block_path.join(&c),
                                                    interner,
                                                )
                                            }),
                                            face.colormap2_mask.as_ref().map(|c| {
                                                EnginePath::from_path(
                                                    &block_path.join(&c),
                                                    interner,
                                                )
                                            }),
                                        ],
                                    };

                                    *mask_to_id.entry(recipe.clone()).or_insert_with(|| {
                                        let idx = block_colormap_masks.len() as u32;
                                        block_colormap_masks.push(recipe);
                                        idx
                                    }) + 1
                                } else {
                                    0
                                };

                                colormap_mask_ids.push(colormap_mask_id);
                            }

                            if texture_ids.len() != colormap_mask_ids.len()
                                && colormap_mask_ids.len() != 0
                            {
                                panic!(
                                    "The texture count should be equal to the colormap mask count if colormaps are used. Faulty project: {}",
                                    project.name
                                );
                            }

                            // we do this cause otherwise texture_ids is out of scope in the metadata part
                            let texture_id_len = texture_ids.len();

                            // if len is 1, use the texture id directly
                            // otherwise, use the variant data
                            // this is basically some union action!! :3 yay yay jump jump
                            // also it's kinda hard to wrap your head around it so don't worry guys
                            if texture_id_len == 1 {
                                texture_or_variant_mapping_table.push(texture_ids[0]);
                            } else {
                                let texture_count = texture_id_len;
                                let variant_table_offset = block_variant_mapping_table.len();
                                let variant_data =
                                    (texture_count as u32) << 28 | variant_table_offset as u32;
                                texture_or_variant_mapping_table.push(variant_data);
                                block_variant_mapping_table.append(&mut texture_ids);
                            }

                            let fully_random_faces_bit =
                                (face_config.fully_random_faces.unwrap_or(false) as u32) << 2;

                            // same deal as the texture ids
                            let metadata = if colormap_mask_ids.len() == 1 {
                                let multiple_textures_bit = (texture_id_len > 1) as u32;
                                let metadata = TextureMetadata {
                                    packed_colormap_ids: pack_colormap_ids(
                                        &face_config,
                                        &colormap_registry,
                                        &project.path,
                                        &interner,
                                    ),
                                    mask_atlas_id: colormap_mask_ids[0],
                                    packed_source_ids_and_flipbits: pack_sources(&face_config),
                                    additional_meta: multiple_textures_bit | fully_random_faces_bit,
                                };
                                metadata
                            } else {
                                let mask_count = colormap_mask_ids.len();
                                let variant_table_offset =
                                    colormap_mask_variant_mapping_table.len();
                                let variant_data =
                                    (mask_count as u32) << 28 | variant_table_offset as u32;

                                colormap_mask_variant_mapping_table.append(&mut colormap_mask_ids);

                                let metadata = TextureMetadata {
                                    packed_colormap_ids: pack_colormap_ids(
                                        &face_config,
                                        &colormap_registry,
                                        &project.path,
                                        &interner,
                                    ),
                                    mask_atlas_id: variant_data,
                                    packed_source_ids_and_flipbits: pack_sources(&face_config),
                                    additional_meta: 3 | fully_random_faces_bit, // 3 because 2 first bits flipped
                                };
                                metadata
                            };

                            metadata_table.push(metadata);
                        }
                    }

                    block_registry.register_block(full_id, block);
                }
            }

            loaded_projects.push(project);
        }

        if include_assets {
            BlockRegistryContext {
                loaded_projects,
                block_registry,
                block_texture_paths,
                block_colormap_masks,
                colormap_texture_paths,
                texture_or_variant_mapping_table,
                metadata_table,
                block_variant_mapping_table,
                colormap_mask_variant_mapping_table,
                colormap_registry: Some(colormap_registry),
            }
        } else {
            BlockRegistryContext {
                loaded_projects,
                block_registry,
                block_texture_paths,
                block_colormap_masks,
                colormap_texture_paths,
                texture_or_variant_mapping_table,
                metadata_table,
                block_variant_mapping_table,
                colormap_mask_variant_mapping_table,
                colormap_registry: None,
            }
        }
    }

    pub fn register_block(&mut self, id: String, block_definition: BlockDefinition) {
        let block_properties: BlockProperties = block_definition.into();

        self.properties.push(block_properties);
        self.id_to_name.push(id.clone());
        self.name_to_id.insert(id, self.properties.len() as u16 - 1);
    }

    pub fn get_all_blocks(&self) -> Vec<(&String, &u16)> {
        self.name_to_id.iter().collect()
    }

    pub fn get_block(&self, name_id: &str) -> Option<&u16> {
        self.name_to_id.get(name_id)
    }

    pub fn get_block_count(&self) -> usize {
        self.name_to_id.len()
    }

    pub fn estimate_heap(&self) -> usize {
        let mut sum = 0;

        for block_properties in &self.properties {
            sum += block_properties.estimate_heap();
        }

        for (name, _) in &self.name_to_id {
            sum += name.capacity() + size_of::<String>() + size_of::<u16>() + 1;
        }

        for name in &self.id_to_name {
            sum += name.capacity() + size_of::<String>();
        }

        sum += size_of::<Vec<BlockProperties>>();
        sum += size_of::<FxHashMap<String, u16>>();

        sum
    }
}
