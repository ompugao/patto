use image::{DynamicImage, GenericImage, GenericImageView, Rgba};
use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::math_render;
use patto::utils::{fetch_google_photos_media, is_google_photos_url};

pub(crate) enum CachedImage {
    Loaded(StatefulProtocol),
    /// Remote image still downloading in the background.
    Pending,
    Failed(String),
}

/// A finished background download: cache key and decoded image or error.
pub(crate) type FetchResult = (String, Result<DynamicImage, String>);

const REMOTE_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REMOTE_TIMEOUT: Duration = Duration::from_secs(20);

fn is_remote(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
}

/// Download and decode a remote image (or a Google Photos share link's
/// thumbnail). Runs on its own thread, never on the render loop.
fn download(src: &str, background: Option<[u8; 3]>) -> Result<DynamicImage, String> {
    let url = if is_google_photos_url(src) {
        fetch_google_photos_media(src)
            .ok_or("no Google Photos thumbnail (offline or not shared?)")?
            .thumbnail_url
    } else {
        src.to_string()
    };
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(REMOTE_CONNECT_TIMEOUT)
        .timeout(REMOTE_TIMEOUT)
        .build()
        .map_err(|e| format!("fetch error: {}", e))?;
    let bytes = client
        .get(&url)
        .send()
        .and_then(|resp| resp.error_for_status())
        .and_then(|resp| resp.bytes())
        .map_err(|e| format!("fetch error: {}", e))?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("decode error: {}", e))?;
    Ok(match background {
        Some(bg) => flatten_alpha(img, bg),
        None => img,
    })
}

/// Composite `img` onto a solid `bg` color if it has an alpha channel.
/// Images without alpha (e.g. JPEG) are returned unchanged.
fn flatten_alpha(img: DynamicImage, bg: [u8; 3]) -> DynamicImage {
    if !img.color().has_alpha() {
        return img;
    }
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (x, y, Rgba([r, g, b, a])) in rgba.enumerate_pixels() {
        let alpha = *a as f32 / 255.0;
        let blend = |src: u8, bg: u8| -> u8 {
            (alpha * src as f32 + (1.0 - alpha) * bg as f32).round() as u8
        };
        out.put_pixel(
            x,
            y,
            image::Rgb([blend(*r, bg[0]), blend(*g, bg[1]), blend(*b, bg[2])]),
        );
    }
    DynamicImage::ImageRgb8(out)
}

/// Self-contained image cache and protocol picker.
///
/// Manages image loading, caching, display height, and fullscreen state
/// without any knowledge of the wider application.
pub(crate) struct ImageCache {
    cache: HashMap<String, CachedImage>,
    picker: Option<Picker>,
    /// Default image height in terminal rows (user-configurable via +/-).
    pub(crate) height_rows: u16,
    /// Per-element row heights keyed by cache key (image src or math content).
    /// Images store `height_rows`; math stores the tight pixel-computed height.
    pub(crate) elem_heights: HashMap<String, u16>,
    /// Source of the image currently shown fullscreen (None = normal view).
    pub(crate) fullscreen_src: Option<String>,
    /// RGB background used when compositing images with transparency.
    /// `None` means pass images through unchanged.
    pub(crate) background_color: Option<[u8; 3]>,
    /// Downloaded remote images, kept across height changes so resizing never
    /// goes back to the network. Dropped only by an explicit reload.
    remote: HashMap<String, DynamicImage>,
    in_flight: HashSet<String>,
    fetch_tx: UnboundedSender<FetchResult>,
    fetch_rx: Option<UnboundedReceiver<FetchResult>>,
}

impl ImageCache {
    pub(crate) fn new(protocol_override: Option<&str>) -> Self {
        let picker = Picker::from_query_stdio().ok().map(|mut p| {
            if let Some(proto_str) = protocol_override {
                let protocol_type = match proto_str.to_lowercase().as_str() {
                    "kitty" => Some(ProtocolType::Kitty),
                    "iterm2" => Some(ProtocolType::Iterm2),
                    "sixel" => Some(ProtocolType::Sixel),
                    "halfblocks" => Some(ProtocolType::Halfblocks),
                    other => {
                        eprintln!("Unknown protocol '{}', using auto-detected protocol", other);
                        None
                    }
                };
                if let Some(pt) = protocol_type {
                    p.set_protocol_type(pt);
                }
            }
            p
        });

        let (fetch_tx, fetch_rx) = unbounded_channel();
        Self {
            cache: HashMap::new(),
            picker,
            height_rows: 10,
            elem_heights: HashMap::new(),
            fullscreen_src: None,
            background_color: Some([255, 255, 255]),
            remote: HashMap::new(),
            in_flight: HashSet::new(),
            fetch_tx,
            fetch_rx: Some(fetch_rx),
        }
    }

    /// Receiver of finished background downloads; the event loop hands each
    /// one to [`ImageCache::finish_fetch`] and redraws.
    pub(crate) fn take_fetch_receiver(&mut self) -> Option<UnboundedReceiver<FetchResult>> {
        self.fetch_rx.take()
    }

    pub(crate) fn finish_fetch(&mut self, (src, result): FetchResult) {
        self.in_flight.remove(&src);
        match result {
            Ok(img) => {
                self.remote.insert(src.clone(), img);
                // Build the protocol lazily on the next load() of this src.
                self.cache.remove(&src);
            }
            Err(e) => {
                self.cache.insert(src, CachedImage::Failed(e));
            }
        }
    }

    /// Load an image into the cache if not already present.
    pub(crate) fn load(&mut self, src: &str, root_dir: &Path) {
        if self.cache.contains_key(src) || self.picker.is_none() {
            return;
        }
        if is_remote(src) {
            if let Some(img) = self.remote.get(src) {
                let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img.clone());
                self.elem_heights.insert(src.to_string(), self.height_rows);
                self.cache.insert(src.to_string(), CachedImage::Loaded(protocol));
            } else {
                self.cache.insert(src.to_string(), CachedImage::Pending);
                if self.in_flight.insert(src.to_string()) {
                    let (src, tx, bg) = (src.to_string(), self.fetch_tx.clone(), self.background_color);
                    std::thread::spawn(move || {
                        let result = download(&src, bg);
                        let _ = tx.send((src, result));
                    });
                }
            }
            return;
        }

        let path = root_dir.join(src);

        match image::open(&path) {
            Ok(img) => {
                let img = if let Some(bg) = self.background_color {
                    flatten_alpha(img, bg)
                } else {
                    img
                };
                let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                self.elem_heights.insert(src.to_string(), self.height_rows);
                self.cache
                    .insert(src.to_string(), CachedImage::Loaded(protocol));
            }
            Err(e) => {
                self.cache
                    .insert(src.to_string(), CachedImage::Failed(e.to_string()));
            }
        }
    }

    /// Get a mutable reference to a cached image entry.
    pub(crate) fn get_mut(&mut self, src: &str) -> Option<&mut CachedImage> {
        self.cache.get_mut(src)
    }

    /// Clear all cached images and their stored heights.
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.remote.clear();
        self.elem_heights.clear();
    }

    pub(crate) fn increase_height(&mut self) {
        self.height_rows = (self.height_rows + 5).min(60);
        self.cache.clear();
    }

    pub(crate) fn decrease_height(&mut self) {
        self.height_rows = (self.height_rows.saturating_sub(5)).max(5);
        self.cache.clear();
    }

    /// Render a LaTeX math expression to an image and cache it.
    ///
    /// The cache key is the raw LaTeX content string. Does nothing when there
    /// is no image protocol picker (text-only terminal).
    pub(crate) fn load_math(&mut self, content: &str) {
        if self.cache.contains_key(content) || self.picker.is_none() {
            return;
        }
        match math_render::render_latex_to_image(content) {
            Ok(img) => {
                // Compute the exact terminal rows this image occupies so we
                // can allocate a tight rect (no blank padding below the formula).
                let cell_h = self.picker.as_ref().unwrap().font_size().height;
                let rows_needed = if cell_h > 0 {
                    ((img.height() as f32 / cell_h as f32).ceil() as u16).max(1)
                } else {
                    self.height_rows
                };
                self.elem_heights.insert(content.to_string(), rows_needed);
                let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                self.cache
                    .insert(content.to_string(), CachedImage::Loaded(protocol));
            }
            Err(e) => {
                self.cache
                    .insert(content.to_string(), CachedImage::Failed(e));
            }
        }
    }
}
