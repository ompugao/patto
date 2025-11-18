mod common;

use common::*;

#[tokio::test]
async fn test_prepare_rename_on_current_file() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Reference to [note_b]\n");
    workspace.create_file("note_b.pn", "Content of note B\n");
    workspace.create_file("note_c.pn", "Also reference [note_b]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri_b = workspace.get_uri("note_b.pn");
    client
        .did_open(uri_b.clone(), "Content of note B\n".to_string())
        .await;

    // Position: line 0, char 0 (not on a WikiLink, should rename current file)
    let response = client.prepare_rename(uri_b, 0, 0).await;

    // Should succeed and return the file name as placeholder
    assert!(response.get("result").is_some(), "prepare_rename failed");
    assert_eq!(
        response["result"]["placeholder"].as_str(),
        Some("note_b"),
        "Should return file name as placeholder"
    );

    println!("✅ Prepare rename on current file test passed");
}

#[tokio::test]
async fn test_rename_current_file() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Reference to [note_b]\n");
    workspace.create_file("note_b.pn", "Content of note B\n");
    workspace.create_file("note_c.pn", "Also reference [note_b]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri_b = workspace.get_uri("note_b.pn");
    client
        .did_open(uri_b.clone(), "Content of note B\n".to_string())
        .await;

    // Rename current file from note_b to renamed_note
    let response = client.rename(uri_b, 0, 0, "renamed_note").await;

    assert!(response.get("result").is_some(), "Rename failed");
    let doc_changes = &response["result"]["documentChanges"];
    assert!(doc_changes.is_array(), "No documentChanges");

    // Should have file rename operation
    assert!(
        assert_has_file_rename(doc_changes, "note_b.pn", "renamed_note.pn"),
        "File rename not found"
    );

    // Should have text edits in referencing files
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[renamed_note]"),
        "Text edit in note_a.pn not found"
    );

    assert!(
        assert_has_text_edit(doc_changes, "note_c.pn", "[renamed_note]"),
        "Text edit in note_c.pn not found"
    );

    // Verify URIs are correct
    let changes_array = doc_changes.as_array().unwrap();
    let rename_op = changes_array
        .iter()
        .find(|c| c.get("kind").and_then(|v| v.as_str()) == Some("rename"))
        .expect("No rename operation found");

    let old_uri = rename_op["oldUri"].as_str().unwrap();
    let new_uri = rename_op["newUri"].as_str().unwrap();

    assert!(old_uri.ends_with("note_b.pn"), "Old URI doesn't end with note_b.pn");
    assert!(new_uri.ends_with("renamed_note.pn"), "New URI doesn't end with renamed_note.pn");

    println!("✅ Rename current file test passed");
}

#[tokio::test]
async fn test_rename_current_file_with_anchors() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Reference to [note_b#section1]\n");
    workspace.create_file("note_b.pn", "Content of note B\n{@anchor section1}\n{@anchor section2}\n");
    workspace.create_file("note_c.pn", "Also [note_b#section2]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri_b = workspace.get_uri("note_b.pn");
    client
        .did_open(uri_b.clone(), "Content of note B\n{@anchor section1}\n{@anchor section2}\n".to_string())
        .await;

    // Rename current file
    let response = client.rename(uri_b, 0, 0, "renamed_note").await;

    assert!(response.get("result").is_some(), "Rename failed");
    let doc_changes = &response["result"]["documentChanges"];

    // Should preserve anchors in text edits
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[renamed_note#section1]"),
        "Anchor not preserved in note_a.pn"
    );

    assert!(
        assert_has_text_edit(doc_changes, "note_c.pn", "[renamed_note#section2]"),
        "Anchor not preserved in note_c.pn"
    );

    println!("✅ Rename current file with anchors test passed");
}
