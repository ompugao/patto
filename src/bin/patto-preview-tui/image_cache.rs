use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use std::collections::HashMap;
use std::path::Path;

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
    pub(crate) height_rows: u16,
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

    /// Clear all cached images.
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    pub(crate) fn increase_height(&mut self) {
        self.height_rows = (self.height_rows + 5).min(60);
        self.cache.clear();
    }

    pub(crate) fn decrease_height(&mut self) {
        self.height_rows = (self.height_rows.saturating_sub(5)).max(5);
        self.cache.clear();
    }
}
