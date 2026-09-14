use crate::api::render::render_note;
use crate::api::types::*;

fn blocks(src: &str) -> Vec<Block> {
    render_note(src.to_string()).blocks
}

fn line_spans(block: &Block) -> &[InlineSpan] {
    match &block.kind {
        BlockKind::Line { spans } => spans,
        other => panic!("expected a line block, got {other:?}"),
    }
}

fn plain_text(spans: &[InlineSpan]) -> String {
    spans
        .iter()
        .map(|s| match s {
            InlineSpan::Text { text } => text.clone(),
            InlineSpan::Decoration { children, .. } => plain_text(children),
            InlineSpan::WikiLink { name, anchor } => match anchor {
                Some(a) => format!("{name}#{a}"),
                None => name.clone(),
            },
            InlineSpan::Url { url, title } => title.clone().unwrap_or_else(|| url.clone()),
            InlineSpan::InlineCode { code } => code.clone(),
            InlineSpan::InlineMath { tex } => tex.clone(),
            InlineSpan::Image { image } => image.src.clone(),
            InlineSpan::Embed { url, title, .. } => title.clone().unwrap_or_else(|| url.clone()),
        })
        .collect()
}

#[test]
fn nesting_becomes_depth() {
    let blocks = blocks("a\n\tb\n\t\tc\nd\n");
    let depths: Vec<u32> = blocks.iter().map(|b| b.depth).collect();
    assert_eq!(depths, vec![0, 1, 2, 0]);
    let rows: Vec<u32> = blocks.iter().map(|b| b.row).collect();
    assert_eq!(rows, vec![0, 1, 2, 3]);
}

#[test]
fn wiki_links_and_anchors_are_separate_spans() {
    let blocks = blocks("see [other] and [note#sec] here\n");
    let spans = line_spans(&blocks[0]);
    assert_eq!(
        spans[1],
        InlineSpan::WikiLink {
            name: "other".to_string(),
            anchor: None
        }
    );
    assert_eq!(
        spans[3],
        InlineSpan::WikiLink {
            name: "note".to_string(),
            anchor: Some("sec".to_string())
        }
    );
}

#[test]
fn self_link_has_an_empty_name() {
    let blocks = blocks("jump to [#target]\n");
    let spans = line_spans(&blocks[0]);
    assert_eq!(
        spans[1],
        InlineSpan::WikiLink {
            name: String::new(),
            anchor: Some("target".to_string())
        }
    );
}

#[test]
fn decorations_nest() {
    let blocks = blocks("x [* bold [/ italic]] y\n");
    let spans = line_spans(&blocks[0]);
    let InlineSpan::Decoration {
        fontsize, children, ..
    } = &spans[1]
    else {
        panic!("expected a decoration, got {:?}", spans[1]);
    };
    assert!(*fontsize > 0);
    assert!(matches!(
        children[1],
        InlineSpan::Decoration { italic: true, .. }
    ));
    assert_eq!(plain_text(spans), "x bold italic y");
}

#[test]
fn anchors_are_recorded_with_their_block_index() {
    let rendered = render_note("first\nsecond  #here\n".to_string());
    assert_eq!(rendered.anchors.len(), 1);
    assert_eq!(rendered.anchors[0].name, "here");
    assert_eq!(rendered.anchors[0].block_index, 1);
    assert_eq!(rendered.blocks[1].anchors, vec!["here".to_string()]);
}

#[test]
fn code_block_keeps_its_lines_and_language() {
    let blocks = blocks("[@code python]\n\tprint(1)\n\tx = 2\n");
    assert_eq!(blocks.len(), 1);
    let BlockKind::Code { lang, lines } = &blocks[0].kind else {
        panic!("expected a code block, got {:?}", blocks[0].kind);
    };
    assert_eq!(lang, "python");
    assert_eq!(lines, &vec!["print(1)".to_string(), "x = 2".to_string()]);
}

#[test]
fn math_block_joins_its_lines() {
    let blocks = blocks("[@math]\n\ta = b\n\tc = d\n");
    let BlockKind::Math { tex } = &blocks[0].kind else {
        panic!("expected a math block, got {:?}", blocks[0].kind);
    };
    assert_eq!(tex, "a = b\nc = d");
}

#[test]
fn quote_lines_carry_quote_depth() {
    let blocks = blocks("[@quote]\n\touter\n\t[@quote]\n\t\tinner\n");
    let depths: Vec<u32> = blocks.iter().map(|b| b.quote_depth).collect();
    assert_eq!(depths, vec![1, 2]);
    assert_eq!(plain_text(line_spans(&blocks[0])), "outer");
    assert_eq!(plain_text(line_spans(&blocks[1])), "inner");
}

#[test]
fn table_cells_keep_inline_spans() {
    let blocks = blocks("[@table cap]\n\th1\th2\n\ta\t[b]\n");
    let BlockKind::Table { caption, rows } = &blocks[0].kind else {
        panic!("expected a table, got {:?}", blocks[0].kind);
    };
    assert_eq!(caption.as_deref(), Some("cap"));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cells.len(), 2);
    assert_eq!(plain_text(&rows[0].cells[0].spans), "h1");
    assert_eq!(
        rows[1].cells[1].spans[0],
        InlineSpan::WikiLink {
            name: "b".to_string(),
            anchor: None
        }
    );
}

#[test]
fn an_image_only_line_becomes_an_image_block() {
    let blocks = blocks("[@img ./local.png \"alt\"]\n");
    let BlockKind::Images { images } = &blocks[0].kind else {
        panic!("expected images, got {:?}", blocks[0].kind);
    };
    assert_eq!(images[0].src, "./local.png");
    assert_eq!(images[0].alt.as_deref(), Some("alt"));
    assert!(images[0].is_local);
}

#[test]
fn a_remote_image_is_not_local() {
    let blocks = blocks("[@img https://example.com/a.png]\n");
    let BlockKind::Images { images } = &blocks[0].kind else {
        panic!("expected images, got {:?}", blocks[0].kind);
    };
    assert!(!images[0].is_local);
}

#[test]
fn an_image_among_text_stays_inline() {
    let blocks = blocks("before [@img ./a.png] after\n");
    let spans = line_spans(&blocks[0]);
    assert!(matches!(spans[1], InlineSpan::Image { .. }));
}

#[test]
fn youtube_embeds_carry_the_video_id() {
    let blocks = blocks("[@embed https://www.youtube.com/watch?v=dQw4w9WgXcQ Title]\n");
    let spans = line_spans(&blocks[0]);
    let InlineSpan::Embed { kind, title, .. } = &spans[0] else {
        panic!("expected an embed, got {:?}", spans[0]);
    };
    assert_eq!(
        kind,
        &EmbedKind::Youtube {
            video_id: "dQw4w9WgXcQ".to_string()
        }
    );
    assert_eq!(title.as_deref(), Some("Title"));
}

#[test]
fn pdf_embeds_are_recognised() {
    let blocks = blocks("[@embed ./paper.pdf Paper]\n");
    let spans = line_spans(&blocks[0]);
    let InlineSpan::Embed { kind, .. } = &spans[0] else {
        panic!("expected an embed, got {:?}", spans[0]);
    };
    assert_eq!(kind, &EmbedKind::Pdf);
}

#[test]
fn inline_code_and_math_are_extracted() {
    let blocks = blocks("a [` let x = 1 `] b [$ x^2 $] c\n");
    let spans = line_spans(&blocks[0]);
    assert_eq!(
        spans[1],
        InlineSpan::InlineCode {
            code: "let x = 1 ".to_string()
        }
    );
    assert_eq!(
        spans[3],
        InlineSpan::InlineMath {
            tex: "x^2 ".to_string()
        }
    );
}

#[test]
fn a_horizontal_rule_is_its_own_block() {
    let blocks = blocks("above\n-----\nbelow\n");
    assert!(matches!(blocks[1].kind, BlockKind::Rule));
}

#[test]
fn empty_lines_become_blank_blocks() {
    let blocks = blocks("a\n\nb\n");
    assert!(matches!(blocks[1].kind, BlockKind::Blank));
}

#[test]
fn multibyte_text_is_sliced_on_character_boundaries() {
    let blocks = blocks("日本語の [リンク] と emoji 🪽 です\n");
    let spans = line_spans(&blocks[0]);
    assert_eq!(plain_text(spans), "日本語の リンク と emoji 🪽 です");
}

#[test]
fn shorthand_and_block_tasks_are_both_recognised() {
    let blocks = blocks("buy milk !2026-12-31\nwrite up {@task status=doing due=2026-01-05}\n");

    let shorthand = blocks[0].task.as_ref().unwrap();
    assert_eq!(shorthand.status, TaskStatus::Todo);
    assert_eq!(shorthand.due.as_ref().unwrap().text, "2026-12-31");
    assert!(shorthand.is_shorthand);

    let longform = blocks[1].task.as_ref().unwrap();
    assert_eq!(longform.status, TaskStatus::Doing);
    assert!(!longform.is_shorthand);
}

#[test]
fn a_task_without_a_deadline_has_no_due_date() {
    let blocks = blocks("something {@task status=todo}\n");
    assert!(blocks[0].task.as_ref().unwrap().due.is_none());
}

#[test]
fn parse_errors_are_reported_and_the_line_still_renders() {
    let rendered = render_note("!2026-01-02 leading shorthand\n".to_string());
    assert_eq!(rendered.errors.len(), 1);
    assert_eq!(rendered.errors[0].row, 0);
    assert_eq!(rendered.blocks.len(), 1);
    assert!(matches!(rendered.blocks[0].kind, BlockKind::Line { .. }));
}

#[test]
fn a_large_note_renders_in_reasonable_time() {
    let mut src = String::new();
    for i in 0..10_000 {
        src.push_str(&format!("line {i} with [a link] and some text\n"));
    }

    let started = std::time::Instant::now();
    let rendered = render_note(src);
    let elapsed = started.elapsed();

    assert_eq!(rendered.blocks.len(), 10_000);
    assert!(
        elapsed.as_secs() < 10,
        "rendering 10k lines took {elapsed:?}"
    );
}
