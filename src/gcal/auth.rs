//! OAuth 2.0 authentication for Google Calendar API.

use crate::gcal::config::GcalConfig;
use anyhow::{Context, Result};
use google_calendar3::oauth2::{self, authenticator::Authenticator};
use google_calendar3::hyper_rustls::HttpsConnector;
use google_calendar3::hyper::client::HttpConnector;
use std::io::{self, Write};
use std::path::Path;
use time::{Duration, OffsetDateTime};

/// The OAuth scopes required for calendar access
/// Using full calendar scope as google-calendar3 defaults to it
const SCOPES: &[&str] = &["https://www.googleapis.com/auth/calendar"];

/// Type alias for our authenticator
pub type GcalAuthenticator = Authenticator<HttpsConnector<HttpConnector>>;

/// Load token from file and refresh if expired
async fn load_or_refresh_token(config: &GcalConfig, creds_path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(creds_path)?;
    let tokens: serde_json::Value = serde_json::from_str(&content)?;
    
    // Get the first token entry
    let token_entry = tokens.get(0).context("No tokens in credentials file")?;
    let token = token_entry.get("token").context("No token field")?;
    
    let access_token = token.get("access_token")
        .and_then(|v| v.as_str())
        .context("No access_token")?;
    let refresh_token = token.get("refresh_token")
        .and_then(|v| v.as_str());
    
    // Check expiry
    let expires_at = token.get("expires_at");
    let is_expired = if let Some(exp) = expires_at {
        if let Some(arr) = exp.as_array() {
            if arr.len() >= 6 {
                let year = arr[0].as_i64().unwrap_or(0) as i32;
                let ordinal = arr[1].as_i64().unwrap_or(1) as u16;
                let hour = arr[2].as_i64().unwrap_or(0) as u8;
                let minute = arr[3].as_i64().unwrap_or(0) as u8;
                let second = arr[4].as_i64().unwrap_or(0) as u8;
                
                if let Ok(date) = time::Date::from_ordinal_date(year, ordinal) {
                    if let Ok(time) = time::Time::from_hms(hour, minute, second) {
                        let exp_time = OffsetDateTime::new_utc(date, time);
                        // Consider expired if within 60 seconds of expiry
                        exp_time - Duration::seconds(60) <= OffsetDateTime::now_utc()
                    } else {
                        true // Can't parse time, assume expired
                    }
                } else {
                    true // Can't parse date, assume expired
                }
            } else {
                true // Invalid format, assume expired
            }
        } else {
            true // Not an array, assume expired
        }
    } else {
        true // No expiry, assume expired
    };
    
    if is_expired {
        log::info!("Token expired, refreshing...");
        if let Some(refresh) = refresh_token {
            return refresh_access_token(config, refresh, creds_path).await;
        } else {
            anyhow::bail!("Token expired and no refresh token available. Please run 'patto-gcal-sync auth' again.");
        }
    }
    
    Ok(access_token.to_string())
}

/// Refresh the access token using refresh token
async fn refresh_access_token(config: &GcalConfig, refresh_token: &str, creds_path: &Path) -> Result<String> {
    let client_id = config.client_id.as_ref().context("Missing client_id")?;
    let client_secret = config.client_secret.as_ref().context("Missing client_secret")?;
    
    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("Failed to refresh token")?;
    
    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Token refresh failed: {}", error_text);
    }
    
    let token_response: serde_json::Value = resp.json().await?;
    let new_access_token = token_response.get("access_token")
        .and_then(|v| v.as_str())
        .context("No access_token in refresh response")?;
    
    // Calculate new expiry
    let expires_in = token_response.get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600);
    let expires_at = OffsetDateTime::now_utc() + Duration::seconds(expires_in);
    
    let expires_at_tuple = [
        expires_at.year(),
        expires_at.ordinal() as i32,
        expires_at.hour() as i32,
        expires_at.minute() as i32,
        expires_at.second() as i32,
        expires_at.nanosecond() as i32,
        expires_at.offset().whole_hours() as i32,
        expires_at.offset().minutes_past_hour() as i32,
        expires_at.offset().seconds_past_minute() as i32,
    ];
    
    // Update token file (keep refresh token)
    let token_data = serde_json::json!([{
        "scopes": SCOPES,
        "token": {
            "access_token": new_access_token,
            "refresh_token": refresh_token,
            "expires_at": expires_at_tuple,
        }
    }]);
    
    std::fs::write(creds_path, serde_json::to_string_pretty(&token_data)?)?;
    log::info!("Token refreshed and saved");
    
    Ok(new_access_token.to_string())
}

/// Get access token for API calls (refreshes if needed)
pub async fn get_access_token(config: &GcalConfig) -> Result<String> {
    let creds_path = GcalConfig::credentials_path()?;
    
    if !creds_path.exists() {
        anyhow::bail!("No credentials found. Please run 'patto-gcal-sync auth' first.");
    }
    
    load_or_refresh_token(config, &creds_path).await
}

/// Authenticate with Google Calendar API.
/// 
/// Loads existing token and refreshes if needed. Does not prompt for interactive auth.
pub async fn authenticate(config: &GcalConfig) -> Result<GcalAuthenticator> {
    let creds_path = GcalConfig::credentials_path()?;
    
    if !creds_path.exists() {
        anyhow::bail!("No credentials found. Please run 'patto-gcal-sync auth' first.");
    }
    
    // Load and potentially refresh the token
    let access_token = load_or_refresh_token(config, &creds_path).await?;
    
    // Use AccessTokenAuthenticator with our token
    let auth = oauth2::AccessTokenAuthenticator::builder(access_token)
        .build()
        .await
        .context("Failed to create authenticator")?;
    
    Ok(auth)
}

/// Perform interactive authentication with manual URL/code display
pub async fn authenticate_interactive(config: &GcalConfig) -> Result<GcalAuthenticator> {
    let creds_path = GcalConfig::credentials_path()?;
    
    // Check if we already have valid credentials
    if creds_path.exists() {
        println!("Found existing credentials, testing...");
        match authenticate(config).await {
            Ok(auth) => {
                if let Ok(_token) = auth.token(SCOPES).await {
                    println!("Existing credentials are valid!");
                    return Ok(auth);
                }
            }
            Err(_) => {}
        }
        println!("Existing credentials expired or invalid, re-authenticating...");
        std::fs::remove_file(&creds_path)?;
    }
    
    // Manual OAuth flow
    let client_id = config.client_id.as_ref()
        .context("Missing client_id")?;
    
    // Build authorization URL
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/auth?client_id={}&redirect_uri=urn:ietf:wg:oauth:2.0:oob&scope={}&response_type=code&access_type=offline",
        urlencoding::encode(client_id),
        urlencoding::encode(SCOPES[0])
    );
    
    println!("\n📋 Please visit this URL to authorize patto:\n");
    println!("{}\n", auth_url);
    println!("After authorizing, Google will show you an authorization code.");
    print!("Enter the code here: ");
    io::stdout().flush()?;
    
    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim();
    
    if code.is_empty() {
        anyhow::bail!("No authorization code provided");
    }
    
    println!("\nExchanging code for token...");
    
    // Exchange code for token
    let client_secret = config.client_secret.as_ref()
        .context("Missing client_secret")?;
    
    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
        ])
        .send()
        .await
        .context("Failed to exchange code for token")?;
    
    if !resp.status().is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Token exchange failed: {}", error_text);
    }
    
    let token_response: serde_json::Value = resp.json().await
        .context("Failed to parse token response")?;
    
    // Calculate expiry time
    let expires_in = token_response["expires_in"].as_i64().unwrap_or(3600);
    let expires_at = OffsetDateTime::now_utc() + Duration::seconds(expires_in);
    
    // yup-oauth2 serializes OffsetDateTime as a tuple: (year, ordinal, hour, minute, second, nanosecond, offset_hours, offset_minutes, offset_seconds)
    let expires_at_tuple = [
        expires_at.year(),
        expires_at.ordinal() as i32,
        expires_at.hour() as i32,
        expires_at.minute() as i32,
        expires_at.second() as i32,
        expires_at.nanosecond() as i32,
        expires_at.offset().whole_hours() as i32,
        expires_at.offset().minutes_past_hour() as i32,
        expires_at.offset().seconds_past_minute() as i32,
    ];
    
    // Save token in yup-oauth2 format: array of {scopes, token}
    let token_data = serde_json::json!([{
        "scopes": SCOPES,
        "token": {
            "access_token": token_response["access_token"],
            "refresh_token": token_response["refresh_token"],
            "expires_at": expires_at_tuple,
        }
    }]);
    
    std::fs::write(&creds_path, serde_json::to_string_pretty(&token_data)?)?;
    println!("✅ Token saved to {:?}", creds_path);
    
    // Now create the authenticator with the saved credentials
    let auth = authenticate(config).await?;
    Ok(auth)
}

/// Check if valid credentials exist
pub fn credentials_exist() -> bool {
    GcalConfig::credentials_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Build the OAuth client secret from config
fn build_client_secret(config: &GcalConfig) -> Result<oauth2::ApplicationSecret> {
    let client_id = config.client_id.as_ref()
        .context("Missing client_id in config. Please add it to [google_calendar] section.")?;
    let client_secret = config.client_secret.as_ref()
        .context("Missing client_secret in config. Please add it to [google_calendar] section.")?;
    
    Ok(oauth2::ApplicationSecret {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        redirect_uris: vec!["urn:ietf:wg:oauth:2.0:oob".to_string()],
        ..Default::default()
    })
}

/// Revoke the stored credentials
pub async fn revoke_credentials() -> Result<()> {
    let creds_path = GcalConfig::credentials_path()?;
    if creds_path.exists() {
        std::fs::remove_file(&creds_path)?;
        log::info!("Credentials revoked successfully");
    } else {
        log::info!("No credentials to revoke");
    }
    Ok(())
}
