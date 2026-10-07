use patto::utils::extract_og_meta;

use crate::api::types::OpenGraphMeta;

/// The Open Graph tags of a page. A page with none yields an empty value, so
/// a caller only has to look at the fields it cares about.
pub fn parse_open_graph(html: &str) -> OpenGraphMeta {
    OpenGraphMeta {
        title: extract_og_meta(html, "og:title"),
        image: extract_og_meta(html, "og:image"),
        description: extract_og_meta(html, "og:description"),
        site_name: extract_og_meta(html, "og:site_name"),
    }
}
