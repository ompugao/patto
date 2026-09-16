mod index_tests;
mod render_tests;
mod store_tests;
mod task_tests;

use std::path::PathBuf;

/// A scratch notes directory that cleans itself up.
pub(crate) struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    pub fn new() -> Self {
        Workspace {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    pub fn root(&self) -> String {
        self.dir.path().to_string_lossy().to_string()
    }

    pub fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn write(&self, rel_path: &str, content: &str) {
        let path = self.dir.path().join(rel_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    pub fn read(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(rel_path)).unwrap()
    }
}
