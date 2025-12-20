// Note operations for patto-mobile
// Read, write, render notes using patto parser and mobile renderer

use crate::renderer::MobileHtmlRenderer;
use patto::parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Rendered note with metadata
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedNote {
    pub path: String,
    pub name: String,
    pub html: String,
    pub raw_content: String,
}

/// Read raw note content
#[tauri::command]
pub fn read_note(root: PathBuf, file_path: String) -> Result<String, String> {
    let full_path = root.join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    fs::read_to_string(&full_path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Write note content
#[tauri::command]
pub fn write_note(root: PathBuf, file_path: String, content: String) -> Result<(), String> {
    let full_path = root.join(&file_path);

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(&full_path, content).map_err(|e| format!("Failed to write file: {}", e))
}

/// Render note to HTML using mobile-optimized renderer
#[tauri::command]
pub fn render_note(root: PathBuf, file_path: String) -> Result<RenderedNote, String> {
    let full_path = root.join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let content =
        fs::read_to_string(&full_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse the content
    let parse_result = parser::parse_text(&content);

    // Render to HTML using mobile renderer
    let renderer = MobileHtmlRenderer::new(Some(root.to_string_lossy().to_string()));
    let html = renderer
        .render(&parse_result.ast)
        .map_err(|e| format!("Failed to render: {}", e))?;

    // Get note name
    let name = full_path
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(RenderedNote {
        path: file_path,
        name,
        html,
        raw_content: content,
    })
}

/// Render content without reading from file (for preview while editing)
#[tauri::command]
pub fn render_content(content: String) -> Result<String, String> {
    // Parse the content
    let parse_result = parser::parse_text(&content);

    // Render to HTML using mobile renderer
    let renderer = MobileHtmlRenderer::new(None);
    renderer
        .render(&parse_result.ast)
        .map_err(|e| format!("Failed to render: {}", e))
}

/// Link information extracted from a note
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInfo {
    pub target: String,               // Target note name or URL
    pub anchor: Option<String>,       // Optional anchor within target
    pub is_external: bool,            // True if URL, false if internal note link
    pub display_text: Option<String>, // Display text if different from target
}

/// Extract all links from a note
#[tauri::command]
pub fn extract_links(root: PathBuf, file_path: String) -> Result<Vec<LinkInfo>, String> {
    let full_path = root.join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let content =
        fs::read_to_string(&full_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse the content
    let parse_result = parser::parse_text(&content);

    let mut links = Vec::new();
    extract_links_from_ast(&parse_result.ast, &mut links);

    Ok(links)
}

fn extract_links_from_ast(node: &parser::AstNode, links: &mut Vec<LinkInfo>) {
    use parser::AstNodeKind;

    match node.kind() {
        AstNodeKind::WikiLink { link, anchor } => {
            links.push(LinkInfo {
                target: link.clone(),
                anchor: anchor.clone(),
                is_external: false,
                display_text: None,
            });
        }
        AstNodeKind::Link { link, title } => {
            links.push(LinkInfo {
                target: link.clone(),
                anchor: None,
                is_external: true,
                display_text: title.clone(),
            });
        }
        _ => {}
    }

    // Recurse into contents
    for child in node.value().contents.lock().unwrap().iter() {
        extract_links_from_ast(child, links);
    }
    // Recurse into children
    for child in node.value().children.lock().unwrap().iter() {
        extract_links_from_ast(child, links);
    }
}

/// Get image as base64 data URL
#[tauri::command]
pub fn get_image_base64(path: String) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err(format!("Image not found: {:?}", path));
    }

    let data = fs::read(path).map_err(|e| format!("Failed to read image: {}", e))?;

    // Determine MIME type from extension
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };

    let base64 = STANDARD.encode(&data);
    Ok(format!("data:{};base64,{}", mime, base64))
}
