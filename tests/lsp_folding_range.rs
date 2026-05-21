mod common;

use common::*;
use tower_lsp::lsp_types::FoldingRangeKind;

// ---------------------------------------------------------------------------
// Helper: sort ranges for deterministic assertions
// ---------------------------------------------------------------------------
fn sorted(
    mut ranges: Vec<tower_lsp::lsp_types::FoldingRange>,
) -> Vec<tower_lsp::lsp_types::FoldingRange> {
    ranges.sort_by_key(|r| (r.start_line, r.end_line));
    ranges
}

// ---------------------------------------------------------------------------
// Basic indentation folding
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_fold_indent_basic() {
    // Line 0: top-level parent
    //   Line 1: child (indent 1)
    //   Line 2: child (indent 1)
    let content = "parent\n\tchild1\n\tchild2\n";

    let mut workspace = TestWorkspace::new();
    workspace.create_file("note.pn", content);

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("note.pn");
    client.did_open(uri.clone(), content.to_string()).await;

    let ranges = client
        .folding_range(uri)
        .await
        .expect("folding_range returned None");
    let ranges = sorted(ranges);

    // The parent line (row 0) should fold over rows 0–2
    assert!(
        ranges.iter().any(|r| r.start_line == 0 && r.end_line == 2),
        "Expected fold from line 0 to line 2; got: {:?}",
        ranges
    );
}

// ---------------------------------------------------------------------------
// Nested indentation folding
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_fold_indent_nested() {
    // row 0: A
    //   row 1: B (child of A)
    //     row 2: C (child of B)
    //     row 3: D (child of B)
    //   row 4: E (child of A)
    let content = "A\n\tB\n\t\tC\n\t\tD\n\tE\n";

    let mut workspace = TestWorkspace::new();
    workspace.create_file("note.pn", content);

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("note.pn");
    client.did_open(uri.clone(), content.to_string()).await;

    let ranges = client
        .folding_range(uri)
        .await
        .expect("folding_range returned None");
    let ranges = sorted(ranges);

    // A folds over rows 0–4
    assert!(
        ranges.iter().any(|r| r.start_line == 0 && r.end_line == 4),
        "Expected fold A: 0–4; got: {:?}",
        ranges
    );
    // B folds over rows 1–3
    assert!(
        ranges.iter().any(|r| r.start_line == 1 && r.end_line == 3),
        "Expected fold B: 1–3; got: {:?}",
        ranges
    );
}

// ---------------------------------------------------------------------------
// Code block folding
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_fold_code_block() {
    // row 0: intro line
    // row 1: [@code rust]
    //   row 2: fn main() {}
    //   row 3: // end
    // row 4: outro line
    let content = "intro\n[@code rust]\n\tfn main() {}\n\t// end\noutro\n";

    let mut workspace = TestWorkspace::new();
    workspace.create_file("note.pn", content);

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("note.pn");
    client.did_open(uri.clone(), content.to_string()).await;

    let ranges = client
        .folding_range(uri)
        .await
        .expect("folding_range returned None");

    // Code block: rows 1–3, kind = Region
    assert!(
        ranges.iter().any(|r| {
            r.start_line == 1 && r.end_line == 3 && r.kind == Some(FoldingRangeKind::Region)
        }),
        "Expected code block fold 1–3 (Region); got: {:?}",
        ranges
    );
}

// ---------------------------------------------------------------------------
// Quote block folding
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_fold_quote_block() {
    // row 0: [@quote]
    //   row 1: first quoted line
    //   row 2: second quoted line
    // row 3: back to normal
    let content = "[@quote]\n\tquote line 1\n\tquote line 2\nnormal\n";

    let mut workspace = TestWorkspace::new();
    workspace.create_file("note.pn", content);

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("note.pn");
    client.did_open(uri.clone(), content.to_string()).await;

    let ranges = client
        .folding_range(uri)
        .await
        .expect("folding_range returned None");

    assert!(
        ranges.iter().any(|r| {
            r.start_line == 0 && r.end_line == 2 && r.kind == Some(FoldingRangeKind::Region)
        }),
        "Expected quote block fold 0–2 (Region); got: {:?}",
        ranges
    );
}

// ---------------------------------------------------------------------------
// Math block folding
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_fold_math_block() {
    let content = "[@math]\n\tx^2 + y^2 = z^2\n\t\\frac{1}{2}\ntext after\n";

    let mut workspace = TestWorkspace::new();
    workspace.create_file("note.pn", content);

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("note.pn");
    client.did_open(uri.clone(), content.to_string()).await;

    let ranges = client
        .folding_range(uri)
        .await
        .expect("folding_range returned None");

    assert!(
        ranges.iter().any(|r| {
            r.start_line == 0 && r.end_line == 2 && r.kind == Some(FoldingRangeKind::Region)
        }),
        "Expected math block fold 0–2 (Region); got: {:?}",
        ranges
    );
}

// ---------------------------------------------------------------------------
// No folds for flat single-level documents
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_fold_flat_no_ranges() {
    let content = "line one\nline two\nline three\n";

    let mut workspace = TestWorkspace::new();
    workspace.create_file("note.pn", content);

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("note.pn");
    client.did_open(uri.clone(), content.to_string()).await;

    let ranges = client.folding_range(uri).await.unwrap_or_default();

    assert!(
        ranges.is_empty(),
        "Expected no folds for flat document; got: {:?}",
        ranges
    );
}
