use super::flavor::*;

/// Markdown renderer options - configured entirely by flavor
#[derive(Debug, Clone)]
pub struct MarkdownRendererOptions {
    /// The markdown flavor (only public configuration)
    pub flavor: MarkdownFlavor,

    // Internal fields - configured by flavor
    pub(crate) wiki_link_format: WikiLinkFormat,
    pub(crate) file_extension: String,
    pub(crate) task_format: TaskFormat,
    pub(crate) anchor_format: AnchorFormat,
    pub(crate) include_frontmatter: bool,
}

impl MarkdownRendererOptions {
    /// Create options from a flavor - the primary constructor
    pub fn new(flavor: MarkdownFlavor) -> Self {
        let (wiki_link_format, task_format, anchor_format, include_frontmatter) = match flavor {
            MarkdownFlavor::Standard => (
                WikiLinkFormat::Markdown,
                TaskFormat::Checkbox,
                AnchorFormat::HtmlAnchor,
                false,
            ),
            MarkdownFlavor::Obsidian => (
                WikiLinkFormat::WikiStyle,
                TaskFormat::ObsidianEmoji,
                AnchorFormat::ObsidianBlock,
                true,
            ),
            MarkdownFlavor::GitHub => (
                WikiLinkFormat::Markdown,
                TaskFormat::Checkbox,
                AnchorFormat::HtmlComment,
                false,
            ),
        };

        Self {
            flavor,
            wiki_link_format,
            file_extension: ".md".to_string(),
            task_format,
            anchor_format,
            include_frontmatter,
        }
    }

    /// Override frontmatter setting
    pub fn with_frontmatter(mut self, include: bool) -> Self {
        self.include_frontmatter = include;
        self
    }

    // Accessor methods for renderer
    pub fn wiki_link_format(&self) -> WikiLinkFormat {
        self.wiki_link_format
    }

    pub fn file_extension(&self) -> &str {
        &self.file_extension
    }

    pub fn task_format(&self) -> TaskFormat {
        self.task_format
    }

    pub fn anchor_format(&self) -> AnchorFormat {
        self.anchor_format
    }

    pub fn include_frontmatter(&self) -> bool {
        self.include_frontmatter
    }
}
