use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::Response,
};

use crate::utils::mime_type_for_path;

use super::server::AppState;

/// A file from the notes directory (images, videos, PDFs), by its path
/// relative to the repository root.
pub async fn serve(AxumPath(path): AxumPath<String>, State(state): State<AppState>) -> Response {
    let decoded_path = urlencoding::decode(&path).unwrap_or_else(|_| path.clone().into());
    let file_path = state.repository.root_dir.join(decoded_path.as_ref());

    let Ok(canonical_base) = std::fs::canonicalize(&state.repository.root_dir) else {
        return text_response(StatusCode::INTERNAL_SERVER_ERROR, "Base directory error");
    };
    let Ok(canonical_file) = std::fs::canonicalize(&file_path) else {
        return text_response(StatusCode::NOT_FOUND, "File not found");
    };
    // Canonical paths resolve `..` and symlinks, so this prefix check is what
    // keeps a request from reading outside the notes directory.
    if !canonical_file.starts_with(&canonical_base) {
        return text_response(StatusCode::FORBIDDEN, "Access denied");
    }
    if !canonical_file.is_file() {
        return text_response(StatusCode::NOT_FOUND, "File not found");
    }

    match tokio::fs::read(&canonical_file).await {
        Ok(contents) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_type_for_path(&canonical_file))
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(contents))
            .unwrap(),
        Err(_) => text_response(StatusCode::INTERNAL_SERVER_ERROR, "Error reading file"),
    }
}

fn text_response(status: StatusCode, body: &'static str) -> Response {
    Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap()
}
