// File listing and metadata for patto-mobile

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Sort options for file listing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    #[default]
    LastModified,
    LastCreated,
    MostLinked,
    Alphabetical,
}

/// File entry with metadata for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub modified_time: u64,
    pub created_time: u64,
    pub backlink_count: u32,
    pub size_bytes: u64,
}

/// List all patto files in a directory with sorting
#[tauri::command]
pub fn list_files(root: PathBuf, sort_by: SortBy) -> Result<Vec<FileEntry>, String> {
    let mut entries = collect_patto_files(&root).map_err(|e| e.to_string())?;

    // Sort based on criteria
    match sort_by {
        SortBy::LastModified => {
            entries.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
        }
        SortBy::LastCreated => {
            entries.sort_by(|a, b| b.created_time.cmp(&a.created_time));
        }
        SortBy::MostLinked => {
            entries.sort_by(|a, b| b.backlink_count.cmp(&a.backlink_count));
        }
        SortBy::Alphabetical => {
            entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
    }

    Ok(entries)
}

/// Collect patto files recursively
fn collect_patto_files(root: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    if !root.is_dir() {
        return Ok(entries);
    }

    collect_patto_files_recursive(root, root, &mut entries)?;

    Ok(entries)
}

fn collect_patto_files_recursive(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<FileEntry>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files and directories
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            collect_patto_files_recursive(root, &path, entries)?;
        } else if path.extension().map(|e| e == "pn").unwrap_or(false) {
            // Get file metadata
            let metadata = fs::metadata(&path)?;

            let modified_time = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let created_time = metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            // Get relative path from root
            let relative_path = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());

            // Get file name without extension
            let name = path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            entries.push(FileEntry {
                path: relative_path,
                name,
                modified_time,
                created_time,
                backlink_count: 0, // Will be populated when repository is initialized
                size_bytes: metadata.len(),
            });
        }
    }

    Ok(())
}

/// Get file details for a specific file
#[tauri::command]
pub fn get_file_info(root: PathBuf, file_path: String) -> Result<FileEntry, String> {
    let full_path = root.join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let metadata = fs::metadata(&full_path).map_err(|e| e.to_string())?;

    let modified_time = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let created_time = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let name = full_path
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(FileEntry {
        path: file_path,
        name,
        modified_time,
        created_time,
        backlink_count: 0,
        size_bytes: metadata.len(),
    })
}

/// Create a new patto file
#[tauri::command]
pub fn create_file(root: PathBuf, name: String) -> Result<FileEntry, String> {
    // Sanitize name and add extension
    let file_name = if name.ends_with(".pn") {
        name
    } else {
        format!("{}.pn", name)
    };

    let full_path = root.join(&file_name);

    // Check if file already exists
    if full_path.exists() {
        return Err(format!("File already exists: {}", file_name));
    }

    // Create empty file
    fs::write(&full_path, "").map_err(|e| format!("Failed to create file: {}", e))?;

    get_file_info(root, file_name)
}

/// Delete a patto file
#[tauri::command]
pub fn delete_file(root: PathBuf, file_path: String) -> Result<(), String> {
    let full_path = root.join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    fs::remove_file(&full_path).map_err(|e| format!("Failed to delete file: {}", e))
}

/// Rename a patto file
#[tauri::command]
pub fn rename_file(root: PathBuf, old_path: String, new_name: String) -> Result<FileEntry, String> {
    let old_full_path = root.join(&old_path);

    if !old_full_path.exists() {
        return Err(format!("File not found: {}", old_path));
    }

    // Construct new path (keep in same directory)
    let new_file_name = if new_name.ends_with(".pn") {
        new_name
    } else {
        format!("{}.pn", new_name)
    };

    let parent = old_full_path.parent().unwrap_or(&root);
    let new_full_path = parent.join(&new_file_name);

    if new_full_path.exists() {
        return Err(format!("File already exists: {}", new_file_name));
    }

    fs::rename(&old_full_path, &new_full_path).map_err(|e| format!("Failed to rename: {}", e))?;

    let new_relative_path = new_full_path
        .strip_prefix(&root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| new_file_name);

    get_file_info(root, new_relative_path)
}
