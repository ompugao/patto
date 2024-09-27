use url::Url;
use reqwest;
use serde_json::Value;
use std::collections::HashMap;

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

pub(crate) fn get_twitter_embed(tweet_url: &str) -> Option<String> {
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

