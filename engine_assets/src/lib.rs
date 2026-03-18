#![feature(iter_collect_into)]

use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
    time::Instant,
};

use dashmap::DashMap;
use image::{DynamicImage, GrayImage, ImageFormat};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use rustc_hash::FxBuildHasher;

use crate::{
    block_registry::BlockRegistry,
    colormap_registry::ColormapRegistry,
    layer_allocator::LayerAllocator,
    misc::{AssetSlopConfig, MaskRecipe, TextureUpdate, Timings},
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
    pub active_projects: Vec<Project>,

    pub block_registry: BlockRegistry,
    pub colormap_registry: ColormapRegistry,

    pub block_path_to_layer: DashMap<PathBuf, u32, FxBuildHasher>,
    pub colormap_path_to_layer: DashMap<PathBuf, u32, FxBuildHasher>,
    pub mask_dependencies: DashMap<PathBuf, Vec<u32>, FxBuildHasher>,
    pub active_mask_recipes: DashMap<u32, MaskRecipe, FxBuildHasher>,

    pub block_upload_queue: Vec<TextureUpdate>,
    pub mask_upload_queue: Vec<TextureUpdate>,
    pub colormap_upload_queue: Vec<TextureUpdate>,

    pub texture_mapping_table: Vec<u32>,
    pub metadata_table: Vec<TextureMetadata>,
    pub texture_variant_mapping_table: Vec<u32>,
    pub colormap_mask_variant_mapping_table: Vec<u32>,

    pub block_allocator: LayerAllocator,
    pub mask_allocator: LayerAllocator,
    pub colormap_allocator: LayerAllocator,

    pub watch_receiver: Receiver<notify::Result<notify::Event>>,
}

impl AssetManager {
    pub fn init(
        load_projects: Option<Vec<String>>,
        load_native_by_default: bool,
    ) -> (AssetManager, Timings) {
        let mut project_timings = Timings::default();
        let start_time = Instant::now();

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

        project_timings.project_finding = start_time.elapsed().as_nanos();
        project_timings.total = project_timings.project_finding;

        let block_path_to_layer = DashMap::with_hasher(FxBuildHasher::default());
        let colormap_path_to_layer = DashMap::with_hasher(FxBuildHasher::default());
        let mask_dependencies: DashMap<PathBuf, Vec<u32>, FxBuildHasher> =
            DashMap::with_hasher(FxBuildHasher::default());
        let active_mask_recipes: DashMap<u32, MaskRecipe, FxBuildHasher> =
            DashMap::with_hasher(FxBuildHasher::default());

        let (
            loaded_projects,
            block_registry,
            block_texture_paths,
            block_colormap_masks,
            colormap_texture_paths,
            texture_mapping_table,
            metadata_table,
            texture_variant_mapping_table,
            colormap_mask_variant_mapping_table,
            colormap_registry,
        ) = BlockRegistry::init(projects_to_load, true);

        project_timings.block_registry_init =
            start_time.elapsed().as_nanos() - project_timings.total;
        project_timings.total += project_timings.block_registry_init;

        let ((block_upload_queue, colormap_upload_queue), mask_upload_queue) = rayon::join(
            || {
                rayon::join(
                    || {
                        block_texture_paths
                            .into_par_iter()
                            .enumerate()
                            .map(|(i, p)| {
                                let bytes = fs::read(&p).expect("Failed to read file");
                                let image =
                                    image::load_from_memory_with_format(&bytes, ImageFormat::Qoi)
                                        .expect("Failed to load block texture");
                                let update = TextureUpdate {
                                    layer_index: i as u32,
                                    data: image,
                                };
                                block_path_to_layer.insert(p, i as u32);
                                update
                            })
                            .collect::<Vec<_>>()
                    },
                    || {
                        let colormap_texture_paths =
                            colormap_texture_paths.into_par_iter().collect::<Vec<_>>();
                        colormap_texture_paths
                            .into_par_iter()
                            .enumerate()
                            .map(|(i, p)| {
                                let bytes = fs::read(&p).expect("Failed to read file");
                                let image =
                                    image::load_from_memory_with_format(&bytes, ImageFormat::Qoi)
                                        .expect("Failed to load block texture");
                                let update = TextureUpdate {
                                    layer_index: i as u32,
                                    data: image,
                                };
                                colormap_path_to_layer.insert(p, i as u32);
                                update
                            })
                            .collect::<Vec<_>>()
                    },
                )
            },
            || {
                block_colormap_masks
                    .into_par_iter()
                    .enumerate()
                    .map(|(i, recipe)| {
                        let layer_idx = i as u32;
                        for path in recipe.paths.iter().flatten() {
                            mask_dependencies
                                .entry(path.clone())
                                .or_default()
                                .push(layer_idx);
                        }
                        active_mask_recipes.insert(layer_idx, recipe.clone());

                        TextureUpdate {
                            layer_index: layer_idx,
                            data: DynamicImage::ImageLuma8(bake_mask_from_recipe(&recipe)),
                        }
                    })
                    .collect::<Vec<_>>()
            },
        );

        project_timings.image_loading = start_time.elapsed().as_nanos() - project_timings.total;
        project_timings.total += project_timings.image_loading;

        let config = AssetSlopConfig::default();

        let block_allocator =
            LayerAllocator::new(block_upload_queue.len() as u32, config.block_padding);
        let mask_allocator =
            LayerAllocator::new(mask_upload_queue.len() as u32, config.mask_padding);
        let colormap_allocator =
            LayerAllocator::new(colormap_upload_queue.len() as u32, config.colormap_padding);

        project_timings.allocator_setup = start_time.elapsed().as_nanos() - project_timings.total;
        project_timings.total += project_timings.allocator_setup;

        let (tx, rx) = channel();
        let watch_paths: Vec<PathBuf> = loaded_projects.iter().map(|p| p.path.clone()).collect();

        std::thread::spawn(move || {
            let mut watcher =
                RecommendedWatcher::new(tx, Config::default()).expect("Failed to create watcher");

            for path in watch_paths {
                let _ = watcher.watch(&path, RecursiveMode::Recursive);
            }

            std::thread::park();
        });

        project_timings.watcher_setup = start_time.elapsed().as_nanos() - project_timings.total;
        project_timings.total += project_timings.watcher_setup;

        let time_elapsed = start_time.elapsed().as_millis();
        println!("Initialization time: {:?}ms", time_elapsed);
        println!("Block count: {:?}", block_registry.get_block_count());

        (
            AssetManager {
                active_projects: loaded_projects,

                block_registry,
                colormap_registry: colormap_registry.unwrap(),

                block_path_to_layer,
                colormap_path_to_layer,
                mask_dependencies,
                active_mask_recipes,

                block_upload_queue,
                mask_upload_queue,
                colormap_upload_queue,

                texture_mapping_table,
                metadata_table,
                texture_variant_mapping_table,
                colormap_mask_variant_mapping_table,

                block_allocator,
                mask_allocator,
                colormap_allocator,

                watch_receiver: rx,
            },
            project_timings,
        )
    }

    pub fn update_assets(&mut self) {
        while let Ok(Ok(event)) = self.watch_receiver.try_recv() {
            if !event.kind.is_modify() {
                continue;
            }

            println!(
                "Hot-reloaded asset: {:?}",
                event.paths[0].file_name().unwrap()
            );

            for path in event.paths {
                if let Some(layer) = self.block_path_to_layer.get(&path).map(|r| *r) {
                    if let Ok(img) = image::open(&path) {
                        self.push_block_update(layer, img); // error 1
                        println!("Hot-reloaded block: {:?}", path.file_name().unwrap());
                    }
                }

                let layers = self.mask_dependencies.get(&path).map(|r| r.clone());

                if let Some(layers) = layers {
                    for layer in layers {
                        if let Some(recipe) =
                            self.active_mask_recipes.get(&layer).map(|r| r.clone())
                        {
                            let baked = bake_mask_from_recipe(&recipe);
                            self.push_mask_update(layer, baked);
                            println!(
                                "Re-baked mask layer {} due to change in {:?}",
                                layer,
                                path.file_name().unwrap()
                            );
                        }
                    }
                }

                if let Some(layer) = self.colormap_path_to_layer.get(&path).map(|r| *r) {
                    if let Ok(img) = image::open(&path) {
                        println!("Hot-reloaded colormap: {:?}", path.file_name().unwrap());
                        self.push_colormap_update(layer, img);
                    }
                }
            }
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

    fn push_block_update(&mut self, layer: u32, data: DynamicImage) {
        if let Some(existing) = self
            .block_upload_queue
            .iter_mut()
            .find(|u| u.layer_index == layer)
        {
            existing.data = data;
        } else {
            self.block_upload_queue.push(TextureUpdate {
                layer_index: layer,
                data,
            });
        }
    }

    fn push_mask_update(&mut self, layer: u32, data: GrayImage) {
        let dynamic_data = DynamicImage::ImageLuma8(data);

        if let Some(existing) = self
            .mask_upload_queue
            .iter_mut()
            .find(|u| u.layer_index == layer)
        {
            existing.data = dynamic_data;
        } else {
            self.mask_upload_queue.push(TextureUpdate {
                layer_index: layer,
                data: dynamic_data,
            });
        }
    }

    fn push_colormap_update(&mut self, layer: u32, data: DynamicImage) {
        if let Some(existing) = self
            .colormap_upload_queue
            .iter_mut()
            .find(|u| u.layer_index == layer)
        {
            existing.data = data;
        } else {
            self.colormap_upload_queue.push(TextureUpdate {
                layer_index: layer,
                data,
            });
        }
    }
}

fn bake_mask_from_recipe(recipe: &MaskRecipe) -> GrayImage {
    let m0 = recipe.paths[0].as_ref().and_then(|p| {
        let bytes = fs::read(&p).expect("Failed to read file");
        image::load_from_memory_with_format(&bytes, ImageFormat::Qoi)
            .ok()
            .map(|i| i.to_luma8())
    });
    let m1 = recipe.paths[1].as_ref().and_then(|p| {
        let bytes = fs::read(&p).expect("Failed to read file");
        image::load_from_memory_with_format(&bytes, ImageFormat::Qoi)
            .ok()
            .map(|i| i.to_luma8())
    });
    let m2 = recipe.paths[2].as_ref().and_then(|p| {
        let bytes = fs::read(&p).expect("Failed to read file");
        image::load_from_memory_with_format(&bytes, ImageFormat::Qoi)
            .ok()
            .map(|i| i.to_luma8())
    });

    blend_masks(&m0, &m1, &m2)
}

/// Quantize the 3 masks to be 3 bits 3 bits 2 bits
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
