use std::collections::HashMap;
use url::Url;

pub fn get_youtube_id(value: &str) -> Option<String> {
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

#[cfg(feature = "oembed")]
pub fn get_twitter_embed(tweet_url: &str) -> Option<String> {
    use serde_json::Value;

    let parsed_url = Url::parse(tweet_url).ok()?;

    match parsed_url.host_str()? {
        "twitter.com" | "x.com" => {
            // Construct the Twitter embed API URL
            let api_url = format!("https://publish.twitter.com/oembed?url={}", tweet_url);

            //Send the request to the API
            let response = reqwest::blocking::get(&api_url).ok()?;

            // Parse the response as JSON
            let json: Value = response.json().ok()?;

            // Check if the JSON contains the 'html' field
            if let Some(html) = json.get("html") {
                return html.as_str().map(|s| s.to_string());
            }
            None
        }
        _ => None, // Return None if the domain is not twitter.com or x.com
    }
}

/// Without the `oembed` feature there is no network stack, so the renderer falls
/// back to emitting a plain link for twitter/x URLs.
#[cfg(not(feature = "oembed"))]
pub fn get_twitter_embed(_tweet_url: &str) -> Option<String> {
    None
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

/// Google serves a stripped share page (without Open Graph tags) to unknown
/// clients, so fetches of Google Photos pages pretend to be a browser.
pub const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

/// Network budget for one Google Photos request, so a stalled or offline
/// network degrades embeds to plain links instead of hanging the preview.
pub const GOOGLE_PHOTOS_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const GOOGLE_PHOTOS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Media behind a public Google Photos share link, scraped from its Open Graph tags.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GooglePhotosMedia {
    pub thumbnail_url: String,
    /// Directly playable mp4 (`=dv`; the `=m18` stream rejects foreign referrers
    /// with 403/429). `None` when the share is a photo or album.
    pub video_url: Option<String>,
    pub title: Option<String>,
}

/// Whether `url` is a Google Photos share link that can be read without signing in.
pub fn is_google_photos_url(url: &str) -> bool {
    let Ok(parsed_url) = Url::parse(url) else {
        return false;
    };
    match parsed_url.host_str() {
        Some("photos.app.goo.gl") => true,
        Some("goo.gl") => parsed_url.path().starts_with("/photos"),
        Some("photos.google.com") => parsed_url.path().contains("/share/"),
        _ => false,
    }
}

/// Content of `<meta property="{prop}" content="...">`, in either attribute order.
pub fn extract_og_meta(html: &str, prop: &str) -> Option<String> {
    let prop = regex::escape(prop);
    let patterns = [
        format!(r#"<meta[^>]*property="{prop}"[^>]*content="([^"]*)""#),
        format!(r#"<meta[^>]*content="([^"]*)"[^>]*property="{prop}""#),
    ];
    patterns.iter().find_map(|pattern| {
        let re = regex::Regex::new(pattern).ok()?;
        let content = re.captures(html)?.get(1)?.as_str();
        Some(
            content
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&"),
        )
    })
}

/// Strip the `=w600-h315-...` sizing suffix of a googleusercontent URL.
fn googleusercontent_base(url: &str) -> &str {
    url.rsplit_once('=').map_or(url, |(base, _)| base)
}

/// Parse a Google Photos share page into its thumbnail (resized to 1280x720)
/// and, for videos, an mp4 stream URL.
pub fn parse_google_photos_page(html: &str) -> Option<GooglePhotosMedia> {
    let image = extract_og_meta(html, "og:image")?;
    let base = googleusercontent_base(&image);
    let video_url = extract_og_meta(html, "og:video")
        .map(|video| format!("{}=dv", googleusercontent_base(&video)));
    Some(GooglePhotosMedia {
        thumbnail_url: format!("{}=w1280-h720-no", base),
        video_url,
        title: extract_og_meta(html, "og:title"),
    })
}

#[cfg(feature = "oembed")]
pub fn fetch_google_photos_media(share_url: &str) -> Option<GooglePhotosMedia> {
    if !is_google_photos_url(share_url) {
        return None;
    }
    let client = reqwest::blocking::Client::builder()
        .user_agent(BROWSER_USER_AGENT)
        .connect_timeout(GOOGLE_PHOTOS_CONNECT_TIMEOUT)
        .timeout(GOOGLE_PHOTOS_TIMEOUT)
        .build()
        .ok()?;
    let html = client.get(share_url).send().ok()?.text().ok()?;
    parse_google_photos_page(&html)
}

/// Without the `oembed` feature there is no network stack to scrape the share page.
#[cfg(not(feature = "oembed"))]
pub fn fetch_google_photos_media(_share_url: &str) -> Option<GooglePhotosMedia> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIDEO_PAGE: &str = r#"<head><meta property="og:title" content="New video · Sunday, Oct 4 &amp; more"><meta property="og:image" content="https://lh3.googleusercontent.com/pw/AP1Gcz-abc_123=w600-h315-p-k-no"><meta property="og:video" content="https://lh3.googleusercontent.com/pw/AP1Gcz-abc_123=w600-h315-k-no-m18"><meta property="og:video:type" content="video/mp4"></head>"#;

    #[test]
    fn test_is_google_photos_url() {
        assert!(is_google_photos_url(
            "https://photos.app.goo.gl/ykDMgCMfBdZhEWGV9"
        ));
        assert!(is_google_photos_url("https://goo.gl/photos/abc"));
        assert!(is_google_photos_url(
            "https://photos.google.com/share/AF1Qip?key=abc"
        ));
        assert!(!is_google_photos_url(
            "https://photos.google.com/photo/AF1Qip"
        ));
        assert!(!is_google_photos_url("https://goo.gl/maps/abc"));
        assert!(!is_google_photos_url("https://www.youtube.com/watch?v=x"));
        assert!(!is_google_photos_url("./local.pdf"));
    }

    #[test]
    fn test_extract_og_meta() {
        assert_eq!(
            extract_og_meta(VIDEO_PAGE, "og:title").as_deref(),
            Some("New video · Sunday, Oct 4 & more")
        );
        // `og:video` must not match `og:video:type`, and vice versa.
        assert_eq!(
            extract_og_meta(VIDEO_PAGE, "og:video:type").as_deref(),
            Some("video/mp4")
        );
        assert_eq!(
            extract_og_meta(r#"<meta content="x" property="og:image">"#, "og:image").as_deref(),
            Some("x")
        );
        assert_eq!(extract_og_meta(VIDEO_PAGE, "og:description"), None);
    }

    #[test]
    fn test_parse_google_photos_page() {
        let media = parse_google_photos_page(VIDEO_PAGE).unwrap();
        assert_eq!(
            media.thumbnail_url,
            "https://lh3.googleusercontent.com/pw/AP1Gcz-abc_123=w1280-h720-no"
        );
        assert_eq!(
            media.video_url.as_deref(),
            Some("https://lh3.googleusercontent.com/pw/AP1Gcz-abc_123=dv")
        );

        let photo_page = r#"<meta property="og:image" content="https://lh3.googleusercontent.com/pw/X=w600-h315">"#;
        let media = parse_google_photos_page(photo_page).unwrap();
        assert_eq!(media.video_url, None);
        assert_eq!(media.title, None);

        assert_eq!(parse_google_photos_page("<html></html>"), None);
    }
}
