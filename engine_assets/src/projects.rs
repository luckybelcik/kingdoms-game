use engine_core::paths::DATA_DIR;
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }

    /// Finds a specific project by name in the data directory.
    pub fn find(name: &str) -> Option<Self> {
        let data_root = DATA_DIR.get()?;
        let path = data_root.join(name);

        if path.is_dir() {
            Some(Self::new(name.to_string(), path))
        } else {
            None
        }
    }

    /// Scans the data directory for all available projects.
    pub fn find_all() -> Vec<Self> {
        let mut projects = Vec::new();
        let Some(data_root) = DATA_DIR.get() else {
            return projects;
        };

        if let Ok(entries) = fs::read_dir(data_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        projects.push(Self::new(name.to_string(), path));
                    }
                }
            }
        }
        projects.sort(); // Ensure deterministic order
        projects
    }

    pub fn estimate_heap(&self) -> usize {
        self.name.capacity() + size_of::<String>() + self.path.capacity() + size_of::<PathBuf>()
    }
}
