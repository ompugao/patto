//! Math rendering: LaTeX/Typst → raster image via the embedded Typst compiler.
//!
//! Public API:
//!  - [`render_typst`]: renders a Typst math expression to an image
//!  - [`render_latex`]: converts LaTeX → Typst via `tex2typst-rs`, then delegates

use image::DynamicImage;
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use typst::LibraryExt;
use typst::World;
use typst_kit::fonts::{FontSearcher, Fonts};

/// Whether to render in display (block) or inline (text) math style.
pub enum MathStyle {
    /// Display style: `$ formula $` — large operators, tall fractions.
    Display,
    /// Inline/text style: `$formula$` — compact fractions, smaller scripts.
    Inline,
}

static MATH_FONTS: OnceLock<Fonts> = OnceLock::new();

fn get_math_fonts() -> &'static Fonts {
    MATH_FONTS.get_or_init(|| {
        let mut searcher = FontSearcher::new();
        searcher.include_system_fonts(false);
        searcher.search()
    })
}

/// Minimal Typst world backed by embedded fonts, for single-expression rendering.
struct MathWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    source: Source,
}

impl MathWorld {
    fn new(source_text: String) -> Self {
        let fonts = get_math_fonts();
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(fonts.book.clone()),
            source: Source::new(main_id, source_text),
        }
    }
}

impl World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        get_math_fonts().fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

/// Render a Typst math expression to a raster image.
///
/// `typst_math` should be valid Typst math content, e.g. `sum_(i=0)^n x_i`.
/// The expression is embedded in a minimal dark-background Typst document and
/// rendered at 2× scale for crispness.
///
/// - [`MathStyle::Display`]: `$ formula $` — full-size operators/fractions.
/// - [`MathStyle::Inline`]: `$formula$` — compact text-style rendering.
pub fn render_typst(typst_math: &str, style: MathStyle) -> Result<DynamicImage, String> {
    let (margins, math_expr) = match style {
        MathStyle::Display => (
            "top: 4pt, bottom: 2pt, left: 4pt, right: 4pt",
            format!("$ {} $", typst_math),
        ),
        MathStyle::Inline => (
            "top: 2pt, bottom: 4pt, left: 3pt, right: 3pt",
            format!("${typst_math}$"),
        ),
    };
    let source_text = format!(
        "#set page(width: auto, height: auto, margin: ({margins}), fill: black)\n\
         #set text(fill: white, size: 14pt)\n\
         {math_expr}\n"
    );

    let world = MathWorld::new(source_text);
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|errors| {
            errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })?;

    let page = document
        .pages
        .first()
        .ok_or_else(|| "Typst produced no pages".to_string())?;

    let pixmap = typst_render::render(page, 2.0);
    let png_bytes = pixmap
        .encode_png()
        .map_err(|e| format!("PNG encode error: {e}"))?;

    image::load_from_memory(&png_bytes).map_err(|e| format!("image decode error: {e}"))
}

/// Render a LaTeX math expression to a raster image.
///
/// Converts LaTeX → Typst math via `tex2typst-rs`, then delegates to
/// [`render_typst`].
pub fn render_latex(latex: &str, style: MathStyle) -> Result<DynamicImage, String> {
    let typst_math =
        tex2typst_rs::tex2typst(latex).map_err(|e| format!("tex2typst conversion failed: {e}"))?;
    render_typst(&typst_math, style)
}
