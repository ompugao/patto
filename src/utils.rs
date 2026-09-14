use std::collections::HashMap;
use std::path::Path;
use url::Url;

pub(crate) fn get_youtube_id(value: &str) -> Option<String> {
    let parsed_url = Url::parse(value).ok()?;

    match parsed_url.host_str()? {
        "youtu.be" => Some(parsed_url.path()[1..].to_string()),

        "www.youtube.com" | "youtube.com" => {
            let path = parsed_url.path();

            if path == "/watch" {
                let query_pairs: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();
                if let Some(video_id) = query_pairs.get("v") {
                    return Some(video_id.to_string());
                }
            } else if path.starts_with("/embed/") || path.starts_with("/v/") {
                let segments: Vec<&str> = path.split('/').collect();
                if segments.len() > 2 {
                    return Some(segments[2].to_string());
                }
            }

            None
        }

        _ => None,
    }
}

pub fn get_gyazo_img_src(url: &str) -> Option<String> {
    let parsed_url = Url::parse(url).ok()?;

    match parsed_url.host_str()? {
        "gyazo.com" => Some(format!(
            "https://i.gyazo.com/{}.png",
            &parsed_url.path()[1..]
        )),
        _ => None,
    }
}

/// MIME type for a file path, based on its extension.
///
/// Used by the preview server to label the files it serves.
pub fn mime_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        // Web formats
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",

        // Image formats
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "ico" => "image/x-icon",
        "heic" => "image/heic",
        "avif" => "image/avif",

        // Video formats
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mkv" => "video/x-matroska",
        "m4v" => "video/x-m4v",

        // Audio formats
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "wma" => "audio/x-ms-wma",

        // Document formats
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "pn" => "text/plain",
        "rtf" => "application/rtf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Programming languages
        "py" => "text/x-python",

        // Archive formats
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",

        _ => "application/octet-stream",
    }
}
