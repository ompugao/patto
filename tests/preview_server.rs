//! The HTTP routes of the browser preview, as patto-preview-ui and the editor
//! clients call them.
#![cfg(feature = "preview")]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use patto::preview::server::{router, AppState};
use patto::repository::Repository;
use reqwest::StatusCode;

struct Server {
    _dir: tempfile::TempDir,
    root: PathBuf,
    addr: SocketAddr,
    base: String,
}

async fn serve() -> Server {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let app = router(AppState::new(Arc::new(Repository::new(root.clone()))));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Server {
        _dir: dir,
        root,
        addr,
        base: format!("http://{addr}"),
    }
}

async fn get(server: &Server, path: &str) -> reqwest::Response {
    reqwest::get(format!("{}{}", server.base, path))
        .await
        .unwrap()
}

/// The status line for `path` sent exactly as written. HTTP clients resolve
/// `..` segments before sending, so a traversal attempt needs a raw socket.
async fn raw_status(server: &Server, path: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = tokio::net::TcpStream::connect(server.addr).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response.lines().next().unwrap_or_default().to_string()
}

fn content_type(response: &reqwest::Response) -> String {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default()
}

#[tokio::test]
async fn the_root_serves_the_bundled_ui() {
    let server = serve().await;
    let response = get(&server, "/").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(content_type(&response), "text/html; charset=utf-8");
    assert!(response.text().await.unwrap().contains("<html"));
}

#[tokio::test]
async fn unknown_paths_fall_back_to_the_ui_for_client_side_routing() {
    let server = serve().await;
    let response = get(&server, "/notes/some-note.pn").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(content_type(&response), "text/html; charset=utf-8");
}

#[tokio::test]
async fn bundled_assets_are_served_with_their_own_content_type() {
    let server = serve().await;
    let response = get(&server, "/vite.svg").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(content_type(&response).starts_with("image/"));
}

#[tokio::test]
async fn files_in_the_notes_directory_are_served_and_cacheable() {
    let server = serve().await;
    std::fs::create_dir(server.root.join("img")).unwrap();
    std::fs::write(server.root.join("img").join("pic.png"), b"png bytes").unwrap();

    let response = get(&server, "/api/files/img/pic.png").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(content_type(&response), "image/png");
    assert_eq!(
        response.headers()[reqwest::header::CACHE_CONTROL],
        "public, max-age=3600"
    );
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"png bytes");
}

#[tokio::test]
async fn file_paths_are_percent_decoded() {
    let server = serve().await;
    std::fs::write(server.root.join("my pic.png"), b"x").unwrap();

    let response = get(&server, "/api/files/my%20pic.png").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn missing_files_and_directories_are_not_found() {
    let server = serve().await;
    std::fs::create_dir(server.root.join("dir")).unwrap();

    assert_eq!(
        get(&server, "/api/files/missing.png").await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&server, "/api/files/dir").await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn files_outside_the_notes_directory_are_denied() {
    let server = serve().await;
    let secret = server
        .root
        .parent()
        .unwrap()
        .join("preview-server-secret.txt");
    std::fs::write(&secret, "secret").unwrap();

    let plain = raw_status(&server, "/api/files/../preview-server-secret.txt").await;
    let encoded = raw_status(&server, "/api/files/%2E%2E/preview-server-secret.txt").await;
    std::fs::remove_file(&secret).unwrap();

    assert_eq!(plain, "HTTP/1.1 403 Forbidden");
    assert_eq!(encoded, "HTTP/1.1 403 Forbidden");
}

#[tokio::test]
async fn embed_proxies_require_a_url_parameter() {
    let server = serve().await;
    for route in [
        "/api/twitter-embed",
        "/api/speakerdeck-embed",
        "/api/slideshare-embed",
        "/api/google-photos-embed",
    ] {
        let response = get(&server, route).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{route}");
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap(),
            serde_json::json!({"error": "Missing url parameter"}),
            "{route}"
        );
    }
}

#[tokio::test]
async fn embed_proxies_reject_urls_from_other_sites() {
    let server = serve().await;
    for (route, message) in [
        ("/api/twitter-embed", "Invalid Twitter URL"),
        ("/api/speakerdeck-embed", "Invalid SpeakerDeck URL"),
        ("/api/slideshare-embed", "Invalid SlideShare URL"),
        ("/api/google-photos-embed", "Invalid Google Photos URL"),
    ] {
        let response = get(&server, &format!("{route}?url=https://example.com/x")).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{route}");
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap(),
            serde_json::json!({"error": message}),
            "{route}"
        );
    }
}
