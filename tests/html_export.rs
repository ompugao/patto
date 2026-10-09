//! Characterisation tests for the HTML renderer.
//!
//! These pin the markup the exporter and the preview both depend on, one test
//! per `AstNodeKind` arm.

use patto::parser;
use patto::renderer::{HtmlRenderer, HtmlRendererOptions, Renderer};

fn render(patto_text: &str) -> String {
    let result = parser::parse_text(patto_text);
    assert!(
        result.parse_errors.is_empty(),
        "Parse errors: {:?}",
        result.parse_errors
    );

    let renderer = HtmlRenderer::new(HtmlRendererOptions {});
    let mut output = Vec::new();
    renderer.format(&result.ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn assert_contains(html: &str, needle: &str) {
    assert!(html.contains(needle), "expected {needle:?} in:\n{html}");
}

#[test]
fn document_wraps_lines_in_a_list() {
    let html = render("one\ntwo\n");
    assert_contains(&html, "<ul class=\"patto-document\">");
    assert_contains(&html, "<li class=\"patto-line\">");
    assert_contains(&html, "one");
    assert_contains(&html, "two");
    assert!(html.ends_with("</ul>"), "unclosed document list:\n{html}");
}

#[test]
fn nested_lines_become_nested_lists() {
    let html = render("parent\n\tchild\n");
    assert_contains(&html, "<ul class=\"patto-children\">");
    assert_contains(&html, "<li class=\"patto-item\">");
}

#[test]
fn text_is_rendered_verbatim() {
    let html = render("hello world\n");
    assert_contains(&html, "hello world");
}

#[test]
fn decoration_carries_style_and_classes() {
    let html = render("[* bold] [/ italic] [- deleted]\n");
    assert_contains(&html, "font-weight: bold;");
    assert_contains(&html, "class=\"italic\"");
    assert_contains(&html, "class=\"patto-deleted\"");
}

#[test]
fn wikilink_points_at_the_note_file() {
    let html = render("[other note]\n");
    assert_contains(
        &html,
        "<a class=\"patto-wikilink\" href=\"other note.pn\">other note</a>",
    );
}

#[test]
fn wikilink_with_anchor_keeps_both_parts() {
    let html = render("[other#section]\n");
    assert_contains(
        &html,
        "<a class=\"patto-wikilink\" href=\"other.pn#section\">other#section</a>",
    );
}

#[test]
fn self_link_is_an_anchor_only() {
    let html = render("[#section]\n");
    assert_contains(
        &html,
        "<a class=\"patto-selflink\" href=\"#section\">#section</a>",
    );
}

#[test]
fn external_link_uses_its_title() {
    let html = render("[https://example.com Example]\n");
    assert_contains(
        &html,
        "<a class=\"patto-link\" href=\"https://example.com\">Example</a>",
    );
}

#[test]
fn external_link_without_title_shows_the_url() {
    let html = render("[https://example.com]\n");
    assert_contains(
        &html,
        "<a class=\"patto-link\" href=\"https://example.com\">https://example.com</a>",
    );
}

#[test]
fn inline_code_is_escaped() {
    let html = render("[` a < b `]\n");
    assert_contains(&html, "<code class=\"patto-inline-code\">");
    assert_contains(&html, "&lt;");
}

#[test]
fn code_block_carries_its_language() {
    let html = render("[@code python]\n\tprint(1)\n");
    assert_contains(&html, "<pre class=\"hljs patto-code-block\">");
    assert_contains(&html, "<code class=\"language-python\">");
    assert_contains(&html, "print(1)");
}

#[test]
fn mermaid_code_block_gets_its_own_tag() {
    let html = render("[@code mermaid]\n\tgraph TD;\n");
    assert_contains(&html, "<pre class=\"mermaid\">");
    assert_contains(&html, "graph TD;");
}

#[test]
fn code_block_body_is_escaped() {
    let html = render("[@code html]\n\t<b>x</b>\n");
    assert_contains(&html, "&lt;b&gt;");
}

#[test]
fn inline_math_is_delimited_for_mathjax() {
    let html = render("[$ x^2 $]\n");
    assert_contains(&html, "<span class=\"patto-math-inline\">\\(");
    assert_contains(&html, "\\)</span>");
}

#[test]
fn math_block_uses_displaylines() {
    let html = render("[@math]\n\tx = 1\n");
    assert_contains(&html, "<div class=\"patto-math-block\">");
    assert_contains(&html, "\\[\\displaylines{");
    assert_contains(&html, "x = 1");
}

#[test]
fn quote_becomes_a_blockquote() {
    let html = render("[@quote]\n\tquoted text\n");
    assert_contains(&html, "<blockquote class=\"patto-quote\">");
    assert_contains(&html, "quoted text");
    assert_contains(&html, "</blockquote>");
}

#[test]
fn nested_quote_content_is_indented() {
    let html = render("[@quote]\n\touter\n\t\tinner\n");
    assert_contains(&html, "<blockquote class=\"patto-quote\">");
    assert_contains(&html, "margin-left: 2em");
}

#[test]
fn table_renders_rows_and_cells() {
    let html = render("[@table caption]\n\ta\tb\n\tc\td\n");
    assert_contains(&html, "<div class=\"patto-table-wrapper\">");
    assert_contains(&html, "<p class=\"patto-table-caption\">caption</p>");
    assert_contains(&html, "<table class=\"patto-table\"><tbody>");
    assert_contains(&html, "<tr><td>a</td><td>b</td></tr>");
    assert_contains(&html, "<tr><td>c</td><td>d</td></tr>");
    assert_contains(&html, "</tbody></table></div>");
}

#[test]
fn horizontal_line_becomes_hr() {
    let html = render("------\n");
    assert_contains(&html, "<hr class=\"patto-hr\"/>");
}

#[test]
fn image_is_wrapped_in_a_figure_linking_the_original() {
    let html = render("[@img ./cat.png]\n");
    assert_contains(&html, "<figure class=\"patto-figure\">");
    assert_contains(&html, "class=\"patto-image-link\" href=\"./cat.png\"");
    assert_contains(&html, "rel=\"noopener noreferrer\"");
    assert_contains(&html, "<img class=\"patto-image\" src=\"./cat.png\"/>");
    assert_contains(&html, "</figure>");
}

#[test]
fn image_alt_becomes_a_caption() {
    let html = render("[@img ./cat.png \"a cat\"]\n");
    assert_contains(&html, "alt=\"a cat\"");
    assert_contains(&html, "<figcaption>a cat</figcaption>");
}

#[test]
fn youtube_embed_becomes_an_iframe() {
    let html = render("[@embed https://www.youtube.com/watch?v=abc123]\n");
    assert_contains(&html, "<div class=\"patto-embed-youtube\">");
    assert_contains(&html, "youtube.com/embed/abc123");
}

#[test]
fn tweet_embed_is_a_placeholder_with_a_link() {
    let html = render("[@embed https://twitter.com/user/status/1]\n");
    assert_contains(&html, "<div class=\"twitter-placeholder\"");
    assert_contains(&html, "data-url=\"https://twitter.com/user/status/1\"");
}

#[test]
fn speakerdeck_embed_is_a_placeholder() {
    let html = render("[@embed https://speakerdeck.com/user/talk]\n");
    assert_contains(&html, "<div class=\"speakerdeck-placeholder\"");
}

#[test]
fn slideshare_embed_is_a_placeholder() {
    let html = render("[@embed https://www.slideshare.net/user/deck]\n");
    assert_contains(&html, "<div class=\"slideshare-placeholder\"");
}

#[test]
fn local_pdf_embed_is_served_through_the_files_api() {
    let html = render("[@embed ./paper.pdf]\n");
    assert_contains(&html, "<div class=\"patto-embed-pdf\"");
    assert_contains(&html, "data-src=\"./paper.pdf\"");
    assert_contains(&html, "href=\"/api/files/./paper.pdf\"");
}

#[test]
fn remote_pdf_embed_keeps_its_url() {
    let html = render("[@embed https://example.com/paper.pdf]\n");
    assert_contains(&html, "data-url=\"https://example.com/paper.pdf\"");
}

#[test]
fn unknown_embed_falls_back_to_a_link() {
    let html = render("[@embed https://example.com/page Title]\n");
    assert_contains(
        &html,
        "<a class=\"patto-link\" href=\"https://example.com/page\">Title</a>",
    );
}

#[test]
fn anchor_gets_an_id_span() {
    let html = render("line #here\n");
    assert_contains(&html, "<span id=\"here\" class=\"anchor\">here</span>");
}

#[test]
fn todo_task_shows_an_icon_and_deadline() {
    let html = render("{@task status=todo due=2024-12-31} buy milk\n");
    assert_contains(&html, "patto-task-icon-todo");
    assert_contains(
        &html,
        "<span class=\"patto-deadline patto-deadline-default\">",
    );
    assert_contains(&html, "2024-12-31");
}

#[test]
fn doing_task_shows_the_doing_icon() {
    let html = render("{@task status=doing due=2024-12-31} write\n");
    assert_contains(&html, "patto-task-icon-doing");
}

#[test]
fn done_task_is_struck_through_and_hides_the_deadline() {
    let html = render("{@task status=done due=2024-12-31} shipped\n");
    assert_contains(&html, "patto-task-icon-done");
    assert_contains(&html, "patto-task-done-text");
    assert!(
        !html.contains("patto-deadline"),
        "a done task should not show a deadline:\n{html}"
    );
}
