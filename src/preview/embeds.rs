//! Proxies for the embeds the browser cannot fetch itself because of CORS.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use axum::{extract::Query, http::StatusCode, response::Json};
use serde_json::{json, Value};

use crate::utils::{self, GooglePhotosMedia};

type JsonResponse = (StatusCode, Json<Value>);
type Params = Query<HashMap<String, String>>;

/// A site with an oEmbed endpoint.
struct OEmbedProvider {
    name: &'static str,
    accepts: fn(&str) -> bool,
    endpoint: fn(&str) -> String,
}

const TWITTER: OEmbedProvider = OEmbedProvider {
    name: "Twitter",
    accepts: |url| url.contains("twitter.com") || url.contains("x.com"),
    endpoint: |url| {
        format!(
            "https://publish.twitter.com/oembed?url={}",
            urlencoding::encode(url)
        )
    },
};

const SPEAKERDECK: OEmbedProvider = OEmbedProvider {
    name: "SpeakerDeck",
    accepts: |url| url.contains("speakerdeck.com"),
    endpoint: |url| {
        format!(
            "https://speakerdeck.com/oembed.json?url={}",
            urlencoding::encode(url)
        )
    },
};

const SLIDESHARE: OEmbedProvider = OEmbedProvider {
    name: "SlideShare",
    accepts: |url| url.contains("slideshare.net"),
    endpoint: |url| {
        format!(
            "https://www.slideshare.net/api/oembed/2?url={}&format=json",
            urlencoding::encode(url)
        )
    },
};

pub async fn twitter(Query(params): Params) -> JsonResponse {
    oembed(&TWITTER, &params).await
}

pub async fn speakerdeck(Query(params): Params) -> JsonResponse {
    oembed(&SPEAKERDECK, &params).await
}

pub async fn slideshare(Query(params): Params) -> JsonResponse {
    oembed(&SLIDESHARE, &params).await
}

async fn oembed(provider: &OEmbedProvider, params: &HashMap<String, String>) -> JsonResponse {
    let Some(url) = params.get("url") else {
        return error(StatusCode::BAD_REQUEST, "Missing url parameter");
    };
    if !(provider.accepts)(url) {
        return error(
            StatusCode::BAD_REQUEST,
            &format!("Invalid {} URL", provider.name),
        );
    }

    let Ok(response) = reqwest::get((provider.endpoint)(url)).await else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to fetch {} embed", provider.name),
        );
    };
    match response.json::<Value>().await {
        Ok(json) => (StatusCode::OK, Json(json)),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to parse {} response", provider.name),
        ),
    }
}

/// Successful Google Photos lookups, keyed by share link. Share links are stable,
/// and re-opening a note would otherwise re-scrape every embed from Google.
static GOOGLE_PHOTOS_CACHE: LazyLock<Mutex<HashMap<String, GooglePhotosMedia>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Shared client with timeouts: on a stalled network a hung lookup would hold
/// one of the browser's few connections and stall every other request.
static GOOGLE_PHOTOS_CLIENT: LazyLock<Option<reqwest::Client>> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(utils::BROWSER_USER_AGENT)
        .connect_timeout(utils::GOOGLE_PHOTOS_CONNECT_TIMEOUT)
        .timeout(utils::GOOGLE_PHOTOS_TIMEOUT)
        .build()
        .ok()
});

/// Google Photos has no oEmbed, so the share page is scraped for its
/// thumbnail and video stream.
pub async fn google_photos(Query(params): Params) -> JsonResponse {
    let Some(url) = params.get("url") else {
        return error(StatusCode::BAD_REQUEST, "Missing url parameter");
    };
    if !utils::is_google_photos_url(url) {
        return error(StatusCode::BAD_REQUEST, "Invalid Google Photos URL");
    }

    if let Some(media) = GOOGLE_PHOTOS_CACHE.lock().unwrap().get(url) {
        return (StatusCode::OK, Json(json!(media)));
    }

    let Some(client) = GOOGLE_PHOTOS_CLIENT.as_ref() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to build HTTP client",
        );
    };

    let Ok(response) = client.get(url).send().await else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch Google Photos page",
        );
    };
    let Ok(html) = response.text().await else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to read Google Photos page",
        );
    };
    let Some(media) = utils::parse_google_photos_page(&html) else {
        return error(
            StatusCode::BAD_GATEWAY,
            "No preview found; is the link shared publicly?",
        );
    };

    let json = json!(media);
    GOOGLE_PHOTOS_CACHE
        .lock()
        .unwrap()
        .insert(url.clone(), media);
    (StatusCode::OK, Json(json))
}

fn error(status: StatusCode, message: &str) -> JsonResponse {
    (status, Json(json!({ "error": message })))
}
