use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use url::Url;

/// Temporary workspace for LSP tests
pub struct TestWorkspace {
    dir: TempDir,
    files: HashMap<String, PathBuf>,
}

impl TestWorkspace {
    /// Create a new temporary workspace
    pub fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp dir");
        Self {
            dir,
            files: HashMap::new(),
        }
    }

    /// Create a .pn file in the workspace
    pub fn create_file(&mut self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, content).expect("Failed to write file");
        self.files.insert(name.to_string(), path.clone());
        path
    }

    /// Get file URI for a file in the workspace
    pub fn get_uri(&self, name: &str) -> Url {
        let path = self
            .files
            .get(name)
            .unwrap_or_else(|| panic!("File not found: {}", name));
        Url::from_file_path(path).expect("Failed to create URI")
    }

    /// Get workspace root URI
    pub fn root_uri(&self) -> Url {
        Url::from_directory_path(self.dir.path()).expect("Failed to create root URI")
    }

    /// Get workspace root path
    pub fn root_path(&self) -> &Path {
        self.dir.path()
    }
}
