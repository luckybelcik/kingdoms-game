use std::path::PathBuf;

use lasso::{Spur, ThreadedRodeo};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EnginePath {
    pub prefix: Spur,
    pub file_name: Spur,
}

impl EnginePath {
    pub fn from_path(path: &PathBuf, interner: &ThreadedRodeo) -> Self {
        let prefix = path
            .parent()
            .map(|p| p.to_string_lossy())
            .unwrap_or_default();
        let file_name = path
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();

        Self {
            prefix: interner.get_or_intern(prefix),
            file_name: interner.get_or_intern(file_name),
        }
    }

    pub fn resolve(&self, interner: &ThreadedRodeo) -> String {
        let pre = interner.resolve(&self.prefix);
        let file = interner.resolve(&self.file_name);
        format!("{}/{}", pre, file)
    }

    pub fn resolve_prefix(&self, interner: &ThreadedRodeo) -> String {
        let pre = interner.resolve(&self.prefix);
        format!("{}", pre)
    }

    pub fn resolve_file(&self, interner: &ThreadedRodeo) -> String {
        let file = interner.resolve(&self.file_name);
        format!("{}", file)
    }
}
