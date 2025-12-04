mod common;

use common::*;

#[tokio::test]
async fn test_anchor_preservation_simple() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "note_a.pn",
        "Simple link [note_b]\nWith anchor [note_b#section1]\n",
    );
    workspace.create_file("note_b.pn", "Content\n{@anchor section1}\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "Simple link [note_b]\nWith anchor [note_b#section1]\n".to_string(),
        )
        .await;

    // Rename note_b -> new_note
    let response = client.rename(uri, 0, 14, "new_note").await;

    assert!(response.get("result").is_some(), "Rename failed");
    let doc_changes = &response["result"]["documentChanges"];

    // Verify simple link
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[new_note]"),
        "Simple link not updated correctly"
    );

    // Verify anchor is preserved
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[new_note#section1]"),
        "Anchor not preserved"
    );

    // Manually check the edits for anchor preservation
    let changes = doc_changes.as_array().unwrap();
    for change in changes {
        if let Some(text_doc) = change.get("textDocument") {
            if let Some(uri_str) = text_doc.get("uri").and_then(|v| v.as_str()) {
                if uri_str.contains("note_a.pn") {
                    if let Some(edits) = change.get("edits").and_then(|v| v.as_array()) {
                        for edit in edits {
                            if let Some(new_text) = edit.get("newText").and_then(|v| v.as_str()) {
                                if new_text.contains("#section1") {
                                    assert!(
                                        assert_anchor_preserved(new_text, "section1"),
                                        "Anchor format incorrect: {}",
                                        new_text
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("✅ Simple anchor preservation test passed");
}

#[tokio::test]
async fn test_anchor_preservation_multiple_anchors() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Link 1 [note_b]\nLink 2 [note_b#section1]\n");
    workspace.create_file(
        "note_c.pn",
        "Link A [note_b]\nLink B [note_b]\nLink C [note_b#section1]\nLink D [note_b#section2]\nLink E [note_b]\n",
    );
    workspace.create_file(
        "note_b.pn",
        "Content\n{@anchor section1}\n{@anchor section2}\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri_a = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri_a.clone(),
            "Link 1 [note_b]\nLink 2 [note_b#section1]\n".to_string(),
        )
        .await;

    // Rename note_b -> renamed
    let response = client.rename(uri_a, 0, 9, "renamed").await;

    assert!(response.get("result").is_some(), "Rename failed");
    let doc_changes = &response["result"]["documentChanges"];

    // Check note_a.pn edits
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[renamed]"),
        "Simple link in note_a not updated"
    );
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[renamed#section1]"),
        "Link with section1 in note_a not updated"
    );

    // Check note_c.pn edits - should have multiple edits preserving different anchors
    assert!(
        assert_has_text_edit(doc_changes, "note_c.pn", "[renamed]"),
        "Simple link in note_c not updated"
    );
    assert!(
        assert_has_text_edit(doc_changes, "note_c.pn", "[renamed#section1]"),
        "Link with section1 in note_c not updated"
    );
    assert!(
        assert_has_text_edit(doc_changes, "note_c.pn", "[renamed#section2]"),
        "Link with section2 in note_c not updated"
    );

    // Count edits in note_c.pn (should be 5: 3 simple + 1 with section1 + 1 with section2)
    let changes = doc_changes.as_array().unwrap();
    for change in changes {
        if let Some(text_doc) = change.get("textDocument") {
            if let Some(uri_str) = text_doc.get("uri").and_then(|v| v.as_str()) {
                if uri_str.contains("note_c.pn") {
                    if let Some(edits) = change.get("edits").and_then(|v| v.as_array()) {
                        assert_eq!(edits.len(), 5, "note_c.pn should have exactly 5 edits");
                    }
                }
            }
        }
    }

    println!("✅ Multiple anchors preservation test passed");
}

#[tokio::test]
async fn test_no_anchor_modification_in_simple_links() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Just a simple [note_b] link\n");
    workspace.create_file("note_b.pn", "Content\n{@anchor section1}\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(uri.clone(), "Just a simple [note_b] link\n".to_string())
        .await;

    // Rename note_b -> new_name
    let response = client.rename(uri, 0, 17, "new_name").await;

    assert!(response.get("result").is_some(), "Rename failed");
    let doc_changes = &response["result"]["documentChanges"];

    // Verify the edit doesn't have an anchor
    let changes = doc_changes.as_array().unwrap();
    for change in changes {
        if let Some(text_doc) = change.get("textDocument") {
            if let Some(uri_str) = text_doc.get("uri").and_then(|v| v.as_str()) {
                if uri_str.contains("note_a.pn") {
                    if let Some(edits) = change.get("edits").and_then(|v| v.as_array()) {
                        for edit in edits {
                            if let Some(new_text) = edit.get("newText").and_then(|v| v.as_str()) {
                                assert!(
                                    !new_text.contains("#"),
                                    "Simple link should not have anchor: {}",
                                    new_text
                                );
                                assert_eq!(new_text, "[new_name]", "Simple link format incorrect");
                            }
                        }
                    }
                }
            }
        }
    }

    println!("✅ No anchor in simple links test passed");
}

#[tokio::test]
async fn test_different_anchors_in_same_file() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "note_a.pn",
        "Intro [note_b#intro]\nMiddle [note_b#middle]\nEnd [note_b#end]\n",
    );
    workspace.create_file(
        "note_b.pn",
        "{@anchor intro}\nContent\n{@anchor middle}\nMore\n{@anchor end}\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "Intro [note_b#intro]\nMiddle [note_b#middle]\nEnd [note_b#end]\n".to_string(),
        )
        .await;

    // Rename note_b -> chapter
    let response = client.rename(uri, 0, 9, "chapter").await;

    assert!(response.get("result").is_some(), "Rename failed");
    let doc_changes = &response["result"]["documentChanges"];

    // All three different anchors should be preserved
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[chapter#intro]"),
        "intro anchor not preserved"
    );
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[chapter#middle]"),
        "middle anchor not preserved"
    );
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "[chapter#end]"),
        "end anchor not preserved"
    );

    println!("✅ Different anchors in same file test passed");
}
