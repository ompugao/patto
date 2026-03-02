use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use std::collections::HashMap;
use std::path::Path;

use crate::math_render;

pub(crate) enum CachedImage {
    Loaded(StatefulProtocol),
    Failed(String),
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

        Self {
            cache: HashMap::new(),
            picker,
            height_rows: 10,
            elem_heights: HashMap::new(),
            fullscreen_src: None,
        }
    }

    /// Load an image into the cache if not already present.
    pub(crate) fn load(&mut self, src: &str, root_dir: &Path) {
        if self.cache.contains_key(src) || self.picker.is_none() {
            return;
        }
        if src.starts_with("http://") || src.starts_with("https://") {
            match reqwest::blocking::get(src) {
                Ok(resp) => match resp.bytes() {
                    Ok(bytes) => match image::load_from_memory(&bytes) {
                        Ok(img) => {
                            let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                            self.elem_heights.insert(src.to_string(), self.height_rows);
                            self.cache
                                .insert(src.to_string(), CachedImage::Loaded(protocol));
                        }
                        Err(e) => {
                            self.cache.insert(
                                src.to_string(),
                                CachedImage::Failed(format!("decode error: {}", e)),
                            );
                        }
                    },
                    Err(e) => {
                        self.cache.insert(
                            src.to_string(),
                            CachedImage::Failed(format!("fetch error: {}", e)),
                        );
                    }
                },
                Err(e) => {
                    self.cache.insert(
                        src.to_string(),
                        CachedImage::Failed(format!("fetch error: {}", e)),
                    );
                }
            }
            return;
        }

        let path = root_dir.join(src);

        match image::open(&path) {
            Ok(img) => {
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
                let (_, cell_h) = self.picker.as_ref().unwrap().font_size();
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
