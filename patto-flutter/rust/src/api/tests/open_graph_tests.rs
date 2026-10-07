use crate::api::open_graph::parse_open_graph;
use crate::api::types::OpenGraphMeta;

const DECK_PAGE: &str = r#"<html><head>
<meta property="og:site_name" content="Speaker Deck" />
<meta property="og:title" content="Rust on Android &amp; iOS" />
<meta content="https://files.speakerdeck.com/presentations/abc/slide_0.jpg?123" property="og:image" />
<meta property="og:description" content="Shipping a shared core" />
</head><body></body></html>"#;

#[test]
fn open_graph_tags_are_read_in_either_attribute_order() {
    let meta = parse_open_graph(DECK_PAGE);
    assert_eq!(
        meta,
        OpenGraphMeta {
            title: Some("Rust on Android & iOS".to_string()),
            image: Some(
                "https://files.speakerdeck.com/presentations/abc/slide_0.jpg?123".to_string()
            ),
            description: Some("Shipping a shared core".to_string()),
            site_name: Some("Speaker Deck".to_string()),
        }
    );
}

#[test]
fn a_page_without_open_graph_tags_is_empty() {
    let meta = parse_open_graph("<html><head><title>plain</title></head></html>");
    assert_eq!(meta, OpenGraphMeta::default());
}

#[test]
fn a_partial_set_of_tags_fills_only_those_fields() {
    let meta = parse_open_graph(r#"<meta property="og:title" content="Only a title">"#);
    assert_eq!(meta.title.as_deref(), Some("Only a title"));
    assert_eq!(meta.image, None);
}
