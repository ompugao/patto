use std::path::Path;

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::Response,
};
use rust_embed::RustEmbed;

use crate::utils::mime_type_for_path;

/// The Vite build of patto-preview-ui, bundled into the binary.
#[derive(RustEmbed)]
#[folder = "patto-preview-ui/dist/"]
struct UiAssets;

const INDEX: &str = "index.html";
const HTML: &str = "text/html; charset=utf-8";

/// A built asset by its path, or `index.html` for any other path so the
/// browser-side router can take over.
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        return index("UI not built — run `npm run build` in patto-preview-ui/");
    }

    match UiAssets::get(path) {
        Some(file) => {
            let content_type = if path.ends_with(".html") {
                HTML
            } else {
                mime_type_for_path(Path::new(path))
            };
            file_response(file.data.to_vec(), content_type)
        }
        None => index("Not found"),
    }
}

fn index(missing_message: &'static str) -> Response {
    match UiAssets::get(INDEX) {
        Some(file) => file_response(file.data.to_vec(), HTML),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(missing_message))
            .unwrap(),
    }
}

fn file_response(data: Vec<u8>, content_type: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(data))
        .unwrap()
}
