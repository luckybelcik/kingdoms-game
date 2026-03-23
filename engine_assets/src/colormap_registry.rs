use std::{collections::HashMap, path::Path, sync::Arc};

use lasso::ThreadedRodeo;
use rustc_hash::FxHashMap;

use crate::engine_path::EnginePath;

pub fn string_to_source_id(s: &str) -> u32 {
    match s {
        "none" => 0,
        "time" => 1,
        "season" => 2,
        "warmth" => 3,
        "humidity" => 4,
        "elevation" => 5,
        "depth" => 6,
        "height" => 7,
        "radius" => 8,
        "skylight" => 9,
        "light" => 10,
        "moonphase" => 11,
        "random_white" => 12,
        "random_perlin" => 13,
        "random_blue" => 14,
        _ => {
            eprintln!("Warning: Unknown source '{}', defaulting to none.", s);
            0
        }
    }
}

#[derive(Debug)]
pub struct ColormapRegistry {
    pub colormaps: FxHashMap<EnginePath, u32>,
    pub unique_images: Vec<EnginePath>,
}

impl Default for ColormapRegistry {
    fn default() -> Self {
        Self {
            colormaps: FxHashMap::default(),
            unique_images: Vec::new(),
        }
    }
}

impl ColormapRegistry {
    pub fn get_or_register_asset(
        &mut self,
        map_name: &str,
        namespace_path: &Path,
        interner: &Arc<ThreadedRodeo>,
    ) -> u32 {
        let full_path = namespace_path.join("textures/colormaps").join(map_name);

        if let Some(&idx) = self
            .colormaps
            .get(&EnginePath::from_path(&full_path, interner))
        {
            idx + 1
        } else {
            let idx = self.unique_images.len() as u32;
            self.unique_images
                .push(EnginePath::from_path(&full_path, interner));
            self.colormaps
                .insert(EnginePath::from_path(&full_path, interner), idx);
            idx + 1
        }
    }

    pub fn get_colormap_id(
        &self,
        map_name: &str,
        namespace_path: &Path,
        interner: &Arc<ThreadedRodeo>,
    ) -> u32 {
        let full_path = namespace_path.join("textures/colormaps").join(map_name);

        // Return index + 1, or 0 if not found
        self.colormaps
            .get(&EnginePath::from_path(&full_path, interner))
            .map(|idx| idx + 1)
            .unwrap_or(0)
    }

    pub fn estimate_heap(&self) -> usize {
        let mut sum = 0;
        for (_, _) in &self.colormaps {
            sum += size_of::<EnginePath>() + size_of::<u32>();
        }
        for _ in &self.unique_images {
            sum += size_of::<EnginePath>();
        }
        sum + size_of::<HashMap<EnginePath, u32>>() + size_of::<Vec<EnginePath>>()
    }
}
