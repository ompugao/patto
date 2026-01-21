mod common;

use common::*;
use tower_lsp::lsp_types::WorkspaceEdit;

#[tokio::test]
async fn test_initialize_lsp() {
    // This test is not needed with InProcessLspClient as initialization is automatic
    println!("✅ Initialize test passed (automatic with InProcessLspClient)");
}

#[tokio::test]
async fn test_prepare_rename_on_wikilink() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "note_a.pn",
        "First line\nSecond line\nThird line\nSee [note_b]\n",
    );
    workspace.create_file("note_b.pn", "Content of note B\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "First line\nSecond line\nThird line\nSee [note_b]\n".to_string(),
        )
        .await;

    // Position: line 3, char 11 (inside [note_b])
    let response = client.prepare_rename(uri, 3, 11).await;

    // Should have a result with range and placeholder
    assert!(response.is_some(), "prepare_rename failed");
    let prepare_result = response.unwrap();
    
    // Check that we got a range and placeholder (PrepareRenameResponse is an enum)
    match prepare_result {
        tower_lsp::lsp_types::PrepareRenameResponse::RangeWithPlaceholder { placeholder, .. } => {
            assert_eq!(placeholder, "note_b", "Wrong placeholder");
        }
        _ => panic!("Expected RangeWithPlaceholder response"),
    }

    println!("✅ Prepare rename test passed");
}

#[tokio::test]
async fn test_rename_note_with_references() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "note_a.pn",
        "First line\nSecond line\nThird line\nSee [note_b]\nAlso [note_b#section1]\n",
    );
    workspace.create_file(
        "note_b.pn",
        "Content of note B\n{@anchor section1}\n{@anchor section2}\n",
    );
    workspace.create_file(
        "note_c.pn",
        "Reference to [note_b]\nAnother [note_b]\nWith anchor [note_b#section2]\nMore [note_b]\nLast [note_b]\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_a = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri_a.clone(),
            "First line\nSecond line\nThird line\nSee [note_b]\nAlso [note_b#section1]\n"
                .to_string(),
        )
        .await;

    // Rename note_b -> project_notes at line 3, char 11
    let response = client.rename(uri_a, 3, 11, "project_notes").await;

    assert!(response.is_some(), "Rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(&workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Should have file rename operation
    assert!(
        assert_has_file_rename(doc_changes, "note_b.pn", "project_notes.pn"),
        "File rename not found"
    );

    // Should have text edits in note_a.pn
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[project_notes]"),
        "Text edit in note_a.pn not found"
    );

    // Should have text edits with anchors preserved
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[project_notes#section1]"),
        "Text edit with anchor in note_a.pn not found"
    );

    // Should have text edits in note_c.pn
    assert!(
        assert_has_text_edit(doc_changes, "note_c.pn", "[project_notes]"),
        "Text edit in note_c.pn not found"
    );

    println!("✅ Rename with references test passed");
}

#[tokio::test]
async fn test_rename_rejects_empty_name() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "See [note_b]\n");
    workspace.create_file("note_b.pn", "Content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(uri.clone(), "See [note_b]\n".to_string())
        .await;

    // Try to rename to empty string
    let response = client.rename(uri, 0, 6, "").await;

    // Should return None (error)
    assert!(response.is_none(), "Should reject empty name");

    println!("✅ Reject empty name test passed");
}

#[tokio::test]
async fn test_rename_rejects_path_separator() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "See [note_b]\n");
    workspace.create_file("note_b.pn", "Content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(uri.clone(), "See [note_b]\n".to_string())
        .await;

    // Try to rename with path separator
    let response = client.rename(uri, 0, 6, "path/to/note").await;

    // Should return None (error)
    assert!(response.is_none(), "Should reject path separator");

    println!("✅ Reject path separator test passed");
}

#[tokio::test]
async fn test_rename_rejects_pn_extension() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "See [note_b]\n");
    workspace.create_file("note_b.pn", "Content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(uri.clone(), "See [note_b]\n".to_string())
        .await;

    // Try to rename with .pn extension
    let response = client.rename(uri, 0, 6, "new_note.pn").await;

    // Should return None (error)
    assert!(response.is_none(), "Should reject .pn extension");

    println!("✅ Reject .pn extension test passed");
}
