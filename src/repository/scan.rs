use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{FileMetadata, Repository, RepositoryMessage};

impl Repository {
    /// Scan the workspace in the background, reporting progress through the
    /// `RepositoryMessage::Scan*` messages.
    pub fn spawn_initial_scan(&self) {
        let repository = self.clone();
        tokio::spawn(async move {
            repository.build_initial_graph().await;
        });
    }

    async fn build_initial_graph(&self) {
        let files = collect_pn_files(&self.root_dir);
        let total = files.len();

        let _ = self
            .tx
            .send(RepositoryMessage::ScanStarted { total_files: total });

        for (idx, file_path) in files.iter().enumerate() {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                self.add_file_to_graph(file_path, &content);
            }

            tokio::task::yield_now().await;
            if (idx + 1) % 5 == 0 || idx == total - 1 {
                let _ = self.tx.send(RepositoryMessage::ScanProgress {
                    scanned: idx + 1,
                    total,
                });
            }
        }

        let _ = self
            .tx
            .send(RepositoryMessage::ScanCompleted { total_files: total });
    }

    /// Every `.pn` file under `dir`, as a path relative to the repository root,
    /// together with its metadata.
    pub fn collect_patto_files_with_metadata(
        &self,
        dir: &Path,
        files: &mut Vec<String>,
        metadata: &mut HashMap<String, FileMetadata>,
    ) {
        walk_pn_files(dir, &mut |path| {
            if let Ok(rel_path) = path.strip_prefix(&self.root_dir) {
                let rel_path_str = rel_path.to_string_lossy().to_string();
                if let Ok(file_metadata) = self.collect_file_metadata(&path) {
                    files.push(rel_path_str.clone());
                    metadata.insert(rel_path_str, file_metadata);
                }
            }
        });
    }

    pub fn collect_file_metadata(&self, file_path: &Path) -> io::Result<FileMetadata> {
        let file_metadata = std::fs::metadata(file_path)?;
        Ok(FileMetadata {
            modified: unix_seconds(file_metadata.modified()),
            created: unix_seconds(file_metadata.created()),
            link_count: self.count_back_links(file_path).try_into().unwrap(),
        })
    }
}

fn unix_seconds(time: io::Result<SystemTime>) -> u64 {
    time.unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn collect_pn_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_pn_files(dir, &mut |path| files.push(path));
    files
}

fn walk_pn_files(dir: &Path, visit: &mut dyn FnMut(PathBuf)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_pn_files(&path, visit);
        } else if path.extension().and_then(|s| s.to_str()) == Some("pn") {
            visit(path);
        }
    }
}
