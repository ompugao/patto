use image::{DynamicImage, GenericImage, GenericImageView, Rgba};
use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use std::collections::HashMap;
use std::path::Path;

use crate::math_render;
use crate::wrap::ElemSizeCache;

pub(crate) enum CachedImage {
    Loaded(StatefulProtocol),
    Failed(String),
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
    /// Cached element sizes (heights and widths) for images and math.
    pub(crate) elem_sizes: ElemSizeCache,
    /// Source of the image currently shown fullscreen (None = normal view).
    pub(crate) fullscreen_src: Option<String>,
    /// RGB background used when compositing images with transparency.
    /// `None` means pass images through unchanged.
    pub(crate) background_color: Option<[u8; 3]>,
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
            elem_sizes: ElemSizeCache::default(),
            fullscreen_src: None,
            background_color: Some([255, 255, 255]),
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
                            let img = if let Some(bg) = self.background_color {
                                flatten_alpha(img, bg)
                            } else {
                                img
                            };
                            let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                            self.elem_sizes
                                .heights
                                .insert(src.to_string(), self.height_rows);
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
                let img = if let Some(bg) = self.background_color {
                    flatten_alpha(img, bg)
                } else {
                    img
                };
                let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                self.elem_sizes
                    .heights
                    .insert(src.to_string(), self.height_rows);
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
        self.elem_sizes.clear();
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
        match math_render::render_latex(content, math_render::MathStyle::Display) {
            Ok(img) => {
                // Compute the exact terminal rows this image occupies so we
                // can allocate a tight rect (no blank padding below the formula).
                let (_, cell_h) = self.picker.as_ref().unwrap().font_size();
                let rows_needed = if cell_h > 0 {
                    ((img.height() as f32 / cell_h as f32).ceil() as u16).max(1)
                } else {
                    self.height_rows
                };
                self.elem_sizes
                    .heights
                    .insert(content.to_string(), rows_needed);
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

    /// Cache key prefix for inline math entries.
    pub(crate) fn inline_math_key(content: &str) -> String {
        format!("__inline__:{}", content)
    }

    /// Render an inline LaTeX math expression to an image and cache it.
    ///
    /// Uses Typst inline/text math style so fractions are compact but readable.
    /// The cache key is `"__inline__:{content}"`. Does nothing when there is no
    /// image protocol picker (text-only terminal).
    pub(crate) fn load_inline_math(&mut self, content: &str) {
        let key = Self::inline_math_key(content);
        if self.cache.contains_key(&key) || self.picker.is_none() {
            return;
        }
        let (cell_w, cell_h) = self.picker.as_ref().unwrap().font_size();
        let cell_width_px = if cell_w > 0 { cell_w as u32 } else { 10 };

        match math_render::render_latex(content, math_render::MathStyle::Inline) {
            Ok(img) => {
                // Compute natural row height (same logic as load_math).
                let rows_needed = if cell_h > 0 {
                    ((img.height() as f32 / cell_h as f32).ceil() as u16).max(1)
                } else {
                    2 // safe fallback for fractions
                };
                self.elem_sizes.heights.insert(key.clone(), rows_needed);
                // Width in terminal columns (round up).
                let cols = ((img.width() as f32 / cell_width_px as f32).ceil() as u16).max(1);
                self.elem_sizes.widths.insert(key.clone(), cols);
                let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                self.cache.insert(key, CachedImage::Loaded(protocol));
            }
            Err(e) => {
                self.cache.insert(key, CachedImage::Failed(e));
            }
        }
    }

    /// Return the column width of a cached inline math image, if available.
    pub(crate) fn inline_math_cols(&self, content: &str) -> Option<u16> {
        let key = Self::inline_math_key(content);
        self.elem_sizes.widths.get(&key).copied()
    }
}
