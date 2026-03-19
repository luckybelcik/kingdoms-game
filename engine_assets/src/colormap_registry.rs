use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;

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

pub struct ColormapRegistry {
    pub colormaps: FxHashMap<String, u32>,
    pub unique_images: Vec<PathBuf>,
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
    pub fn get_or_register_asset(&mut self, map_name: &str, namespace_path: &Path) -> u32 {
        let full_path = namespace_path.join("textures/colormaps").join(map_name);
        let path_str = full_path.to_string_lossy().to_string();

        if let Some(&idx) = self.colormaps.get(&path_str) {
            idx + 1
        } else {
            let idx = self.unique_images.len() as u32;
            self.unique_images.push(full_path);
            self.colormaps.insert(path_str, idx);
            idx + 1
        }
    }

    pub fn get_colormap_id(&self, map_name: &str, namespace_path: &Path) -> u32 {
        let full_path = namespace_path.join("textures/colormaps").join(map_name);
        let path_str = full_path.to_string_lossy().to_string();

        // Return index + 1, or 0 if not found
        self.colormaps
            .get(&path_str)
            .map(|idx| idx + 1)
            .unwrap_or(0)
    }

    pub fn estimate_heap(&self) -> usize {
        let mut sum = 0;
        for (name, _) in &self.colormaps {
            sum += name.capacity() + size_of::<String>() + size_of::<u32>();
        }
        for path in &self.unique_images {
            sum += path.capacity() + size_of::<PathBuf>();
        }
        sum
    }
}
