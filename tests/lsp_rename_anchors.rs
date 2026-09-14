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

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "Simple link [note_b]\nWith anchor [note_b#section1]\n".to_string(),
        )
        .await;

    // Rename note_b -> new_note
    let response = client.rename(uri, 0, 14, "new_note").await;

    assert!(response.is_some(), "Rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

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

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_a = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri_a.clone(),
            "Link 1 [note_b]\nLink 2 [note_b#section1]\n".to_string(),
        )
        .await;

    // Rename note_b -> renamed
    let response = client.rename(uri_a, 0, 9, "renamed").await;

    assert!(response.is_some(), "Rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

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

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(uri.clone(), "Just a simple [note_b] link\n".to_string())
        .await;

    // Rename note_b -> new_name
    let response = client.rename(uri, 0, 17, "new_name").await;

    assert!(response.is_some(), "Rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

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

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "Intro [note_b#intro]\nMiddle [note_b#middle]\nEnd [note_b#end]\n".to_string(),
        )
        .await;

    // Rename note_b -> chapter
    let response = client.rename(uri, 0, 9, "chapter").await;

    assert!(response.is_some(), "Rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

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

// ==========================================
// Tests for Anchor Renaming (renaming anchor definitions)
// ==========================================

#[tokio::test]
async fn test_prepare_rename_on_anchor_definition() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Content here\n#section1\nMore content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "Content here\n#section1\nMore content\n".to_string(),
        )
        .await;

    // Position cursor on #section1 (line 1, character 1 which is inside #section1)
    let response = client.prepare_rename(uri, 1, 1).await;

    assert!(response.is_some(), "prepare_rename failed");
    let prepare_result = response.unwrap();

    match prepare_result {
        tower_lsp::lsp_types::PrepareRenameResponse::RangeWithPlaceholder {
            placeholder, ..
        } => {
            assert_eq!(placeholder, "section1", "Wrong placeholder for anchor");
        }
        _ => panic!("Expected RangeWithPlaceholder response"),
    }

    println!("✅ Prepare rename on anchor definition test passed");
}

#[tokio::test]
async fn test_prepare_rename_on_anchor_long_form() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "note_a.pn",
        "Content here\n{@anchor section1}\nMore content\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("note_a.pn");
    client
        .did_open(
            uri.clone(),
            "Content here\n{@anchor section1}\nMore content\n".to_string(),
        )
        .await;

    // Position cursor on {@anchor section1} (line 1, character 10 which is inside "section1")
    let response = client.prepare_rename(uri, 1, 10).await;

    assert!(
        response.is_some(),
        "prepare_rename failed for long form anchor"
    );
    let prepare_result = response.unwrap();

    match prepare_result {
        tower_lsp::lsp_types::PrepareRenameResponse::RangeWithPlaceholder {
            placeholder, ..
        } => {
            assert_eq!(
                placeholder, "section1",
                "Wrong placeholder for long form anchor"
            );
        }
        _ => panic!("Expected RangeWithPlaceholder response"),
    }

    println!("✅ Prepare rename on long form anchor definition test passed");
}

#[tokio::test]
async fn test_rename_anchor_simple() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Content\n#old_anchor\nMore content\n");
    workspace.create_file("note_b.pn", "Link to [note_a#old_anchor]\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_a = workspace.get_uri("note_a.pn");
    let uri_b = workspace.get_uri("note_b.pn");

    client
        .did_open(
            uri_a.clone(),
            "Content\n#old_anchor\nMore content\n".to_string(),
        )
        .await;
    client
        .did_open(uri_b.clone(), "Link to [note_a#old_anchor]\n".to_string())
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Rename anchor: position on #old_anchor (line 1, char 1)
    let response = client.rename(uri_a, 1, 1, "new_anchor").await;

    assert!(response.is_some(), "Rename failed: {:?}", response);
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify anchor definition is updated in note_a.pn
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "#new_anchor"),
        "Anchor definition not updated"
    );

    // Verify link is updated in note_b.pn
    assert!(
        assert_has_text_edit(doc_changes, "note_b.pn", "[note_a#new_anchor]"),
        "Link with anchor not updated"
    );

    println!("✅ Simple anchor rename test passed");
}

#[tokio::test]
async fn test_rename_anchor_long_form() {
    // Test the long form {@anchor name} syntax
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_a.pn", "Content\n{@anchor old_anchor}\nMore content\n");
    workspace.create_file("note_b.pn", "Link to [note_a#old_anchor]\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_a = workspace.get_uri("note_a.pn");
    let uri_b = workspace.get_uri("note_b.pn");

    client
        .did_open(
            uri_a.clone(),
            "Content\n{@anchor old_anchor}\nMore content\n".to_string(),
        )
        .await;
    client
        .did_open(uri_b.clone(), "Link to [note_a#old_anchor]\n".to_string())
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Rename anchor: position inside {@anchor old_anchor} (line 1, char 10 which is inside "old_anchor")
    let response = client.rename(uri_a, 1, 10, "new_anchor").await;

    assert!(response.is_some(), "Rename failed: {:?}", response);
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify anchor definition is updated (should preserve long form)
    assert!(
        assert_has_text_edit(doc_changes, "note_a.pn", "{@anchor new_anchor}"),
        "Long form anchor definition not updated"
    );

    // Verify link is updated in note_b.pn
    assert!(
        assert_has_text_edit(doc_changes, "note_b.pn", "[note_a#new_anchor]"),
        "Link with anchor not updated"
    );

    println!("✅ Long form anchor rename test passed");
}

#[tokio::test]
async fn test_rename_anchor_multiple_references() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("target.pn", "Content\n#myanchor\nMore content\n");
    workspace.create_file("ref1.pn", "See [target#myanchor] for details\n");
    workspace.create_file(
        "ref2.pn",
        "Also [target#myanchor] and [target#myanchor] again\n",
    );
    workspace.create_file("ref3.pn", "Just [target] no anchor\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_target = workspace.get_uri("target.pn");

    client
        .did_open(
            uri_target.clone(),
            "Content\n#myanchor\nMore content\n".to_string(),
        )
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Rename anchor
    let response = client.rename(uri_target, 1, 1, "renamed_anchor").await;

    assert!(response.is_some(), "Rename failed: {:?}", response);
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify anchor definition updated
    assert!(
        assert_has_text_edit(doc_changes, "target.pn", "#renamed_anchor"),
        "Anchor definition not updated"
    );

    // Verify ref1.pn updated
    assert!(
        assert_has_text_edit(doc_changes, "ref1.pn", "[target#renamed_anchor]"),
        "ref1.pn link not updated"
    );

    // Verify ref2.pn updated (both occurrences)
    assert!(
        assert_has_text_edit(doc_changes, "ref2.pn", "[target#renamed_anchor]"),
        "ref2.pn links not updated"
    );

    // Verify ref3.pn is NOT in the changes (no anchor reference)
    let changes = doc_changes.as_array().unwrap();
    let ref3_changed = changes.iter().any(|change| {
        change
            .get("textDocument")
            .and_then(|td| td.get("uri"))
            .and_then(|u| u.as_str())
            .map(|s| s.contains("ref3.pn"))
            .unwrap_or(false)
    });
    assert!(
        !ref3_changed,
        "ref3.pn should not be modified (has no anchor reference)"
    );

    println!("✅ Multiple references anchor rename test passed");
}

#[tokio::test]
async fn test_rename_anchor_does_not_affect_other_anchors() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("target.pn", "#anchor1\nContent\n#anchor2\n");
    workspace.create_file("ref.pn", "[target#anchor1]\n[target#anchor2]\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_target = workspace.get_uri("target.pn");
    let uri_ref = workspace.get_uri("ref.pn");

    client
        .did_open(
            uri_target.clone(),
            "#anchor1\nContent\n#anchor2\n".to_string(),
        )
        .await;
    client
        .did_open(
            uri_ref.clone(),
            "[target#anchor1]\n[target#anchor2]\n".to_string(),
        )
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Rename only anchor1 (line 0, char 1)
    let response = client.rename(uri_target, 0, 1, "new_anchor1").await;

    assert!(response.is_some(), "Rename failed: {:?}", response);
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify anchor1 is updated
    assert!(
        assert_has_text_edit(doc_changes, "target.pn", "#new_anchor1"),
        "anchor1 not updated"
    );

    // Verify link to anchor1 is updated
    assert!(
        assert_has_text_edit(doc_changes, "ref.pn", "[target#new_anchor1]"),
        "Link to anchor1 not updated"
    );

    // Verify anchor2 and its reference are NOT changed
    // Check that no edit contains "anchor2" being changed
    let changes = doc_changes.as_array().unwrap();
    for change in changes {
        if let Some(edits) = change.get("edits").and_then(|e| e.as_array()) {
            for edit in edits {
                if let Some(new_text) = edit.get("newText").and_then(|t| t.as_str()) {
                    assert!(
                        !new_text.contains("anchor2") || new_text == "[target#anchor2]",
                        "anchor2 should not be modified: found '{}'",
                        new_text
                    );
                }
            }
        }
    }

    println!("✅ Anchor rename isolation test passed");
}

#[tokio::test]
async fn test_multibyte_note_and_anchor_rename() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("リンク元.pn", "Link to [ノート#セクション]\n");
    workspace.create_file("ノート.pn", "Content\n#セクション\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_src = workspace.get_uri("リンク元.pn");
    let uri_note = workspace.get_uri("ノート.pn");

    client
        .did_open(uri_src.clone(), "Link to [ノート#セクション]\n".to_string())
        .await;
    client
        .did_open(uri_note.clone(), "Content\n#セクション\n".to_string())
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 1. Rename Note "ノート" -> "メモ" (from reference in リンク元.pn)
    // "Link to [ノート#セクション]"
    // "Link to [" is 9 characters (ASCII)
    let response = client.rename(uri_src.clone(), 0, 9, "メモ").await;

    assert!(response.is_some(), "Note rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify file rename
    assert!(
        assert_has_file_rename(doc_changes, "ノート.pn", "メモ.pn"),
        "File rename not found in changes"
    );

    // Verify reference update
    assert!(
        assert_has_text_edit(doc_changes, "リンク元.pn", "[メモ#セクション]"),
        "Link to multi-byte note not updated"
    );

    // 2. Rename Anchor "#セクション" -> "#部分" (from definition in ノート.pn)
    // "Content\n#セクション\n" -> Line 1, char 1 (after '#')
    let response = client.rename(uri_note.clone(), 1, 1, "部分").await;

    assert!(response.is_some(), "Anchor rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify definition update
    assert!(
        assert_has_text_edit(doc_changes, "ノート.pn", "#部分"),
        "Multi-byte anchor definition not updated"
    );

    // Verify reference update (assuming starting state since we didn't apply previous edits)
    // The reference in 'リンク元.pn' is '[ノート#セクション]'
    assert!(
        assert_has_text_edit(doc_changes, "リンク元.pn", "[ノート#部分]"),
        "Link to multi-byte anchor not updated: {:?}",
        doc_changes
    );

    println!("✅ Multi-byte note and anchor rename test passed");
}

#[tokio::test]
async fn test_multibyte_long_rename() {
    let mut workspace = TestWorkspace::new();
    let note_name = "長い日本語のファイル名_1234567890";
    let anchor_name = "長いアンカー名_section_with_emoji_🧩";

    // Note A refers to Note B with anchor
    let content_a = format!("Link to [{note_name}#{anchor_name}]\n");
    workspace.create_file("source.pn", &content_a);

    // Note B definition
    let content_b = format!("Content\n#{anchor_name}\nMore Content\n");
    let note_path = workspace.create_file(&format!("{}.pn", note_name), &content_b);

    assert!(note_path.exists(), "Note file was not created");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_src = workspace.get_uri("source.pn");
    let uri_note = workspace.get_uri(&format!("{}.pn", note_name));

    client.did_open(uri_src.clone(), content_a.clone()).await;
    client.did_open(uri_note.clone(), content_b.clone()).await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 1. Rename Note from "source.pn"
    // "Link to [" is 9 bytes/chars (ASCII)
    // Cursor at 10 (first char of name) to be safe.
    let new_note_name = "さらに長い新しい日本語のファイル名_modified_🚀";

    let response = client.rename(uri_src.clone(), 0, 10, new_note_name).await;

    assert!(response.is_some(), "Long note rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify file rename
    assert!(
        assert_has_file_rename(
            doc_changes,
            &format!("{}.pn", note_name),
            &format!("{}.pn", new_note_name)
        ),
        "File rename not found for long note name"
    );

    // Verify reference update
    assert!(
        assert_has_text_edit(
            doc_changes,
            "source.pn",
            &format!("[{new_note_name}#{anchor_name}]")
        ),
        "Link to long note not updated correctly"
    );

    // 2. Rename Anchor must track the rename conceptually but here we operate on old state unless we apply edits.
    let new_anchor_name = "変更されたアンカー_truncated";
    let response = client.rename(uri_note.clone(), 1, 1, new_anchor_name).await;

    assert!(response.is_some(), "Long anchor rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify definition update
    assert!(
        assert_has_text_edit(
            doc_changes,
            &format!("{}.pn", note_name),
            &format!("#{new_anchor_name}")
        ),
        "Long anchor definition not updated"
    );

    // Verify reference update
    assert!(
        assert_has_text_edit(
            doc_changes,
            "source.pn",
            &format!("[{note_name}#{new_anchor_name}]")
        ),
        "Link to long anchor not updated"
    );

    println!("✅ Long multi-byte note and anchor rename test passed");
}

#[tokio::test]
async fn test_multiline_content_rename() {
    let mut workspace = TestWorkspace::new();

    // Create a note with multiple lines and sections
    let note_name = "multiline_note";
    let anchor_name = "target_section";
    let mut note_content = String::new();
    note_content.push_str("# Multiline Note Title\n\n");
    for i in 1..15 {
        note_content.push_str(&format!("This is line {} of padding text.\n", i));
    }
    note_content.push_str(&format!("#{}\n", anchor_name));
    note_content.push_str("Content of the target section.\n");
    for i in 16..30 {
        note_content.push_str(&format!("This is footer line {}.\n", i));
    }

    workspace.create_file(&format!("{}.pn", note_name), &note_content);

    // Create source file ensuring link is in the middle
    let mut src_content = String::new();
    src_content.push_str("# Source Content\n");
    for i in 1..10 {
        src_content.push_str(&format!("Source padding line {}.\n", i));
    }
    // Line 10 (0-indexed) will be the link
    // "Check " is 6 chars. `[` is at 6. Name starts at 7.
    let link_line_prefix = "Check ";
    src_content.push_str(&format!(
        "Check [{}#{}] for details.\n",
        note_name, anchor_name
    ));

    for i in 11..20 {
        src_content.push_str(&format!("Source footer line {}.\n", i));
    }

    workspace.create_file("source.pn", &src_content);

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri_src = workspace.get_uri("source.pn");
    client.did_open(uri_src.clone(), src_content).await;

    // Wait for scanning
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 1. Rename the Note
    // Cursor position: Line 10 (0-indexed).
    // Intro lines: Heading(1) + 9 lines (1..10) = 10 lines. Indices 0..9.
    // So link is at line 10 (index 10).
    // Col: "Check " length is 6. `[` is 6. Name starts at 7.
    let rename_col = link_line_prefix.len() + 1; // 7
    let new_note_name = "renamed_multiline_note";

    let response = client
        .rename(uri_src.clone(), 10, rename_col as u32, new_note_name)
        .await;

    assert!(response.is_some(), "Note rename failed");
    let workspace_edit = response.unwrap();
    let doc_changes_value = serde_json::to_value(workspace_edit.document_changes.unwrap()).unwrap();
    let doc_changes = &doc_changes_value;

    // Verify file rename
    assert!(
        assert_has_file_rename(
            doc_changes,
            &format!("{}.pn", note_name),
            &format!("{}.pn", new_note_name)
        ),
        "File rename failed"
    );

    // Verify text edit in source
    assert!(
        assert_has_text_edit(
            doc_changes,
            "source.pn",
            &format!("[{new_note_name}#{anchor_name}]")
        ),
        "Link text update failed for note rename"
    );

    // 2. Rename the Anchor
    let new_anchor_name = "renamed_section";
    // Renaming anchor from reference is not supported by the server yet.
    // We must rename from the definition.

    // Open the note file
    let uri_note = workspace.get_uri(&format!("{}.pn", note_name));
    client.did_open(uri_note.clone(), note_content).await;

    // Anchor definition is at line 16.
    // "#target_section"
    // # is 0. Name starts at 1.
    let anchor_def_line = 16;
    let anchor_def_col = 1;

    // Wait a bit to ensure note is processed (though did_open should be fast)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let response_anchor = client
        .rename(
            uri_note.clone(),
            anchor_def_line,
            anchor_def_col,
            new_anchor_name,
        )
        .await;

    assert!(response_anchor.is_some(), "Anchor rename failed");
    let workspace_edit_anchor = response_anchor.unwrap();
    let doc_changes_anchor_value =
        serde_json::to_value(workspace_edit_anchor.document_changes.unwrap()).unwrap();
    let doc_changes_anchor = &doc_changes_anchor_value;

    // Verify definition update in note file
    assert!(
        assert_has_text_edit(
            doc_changes_anchor,
            &format!("{}.pn", note_name),
            &format!("#{new_anchor_name}")
        ),
        "Anchor definition update failed"
    );

    // Verify usage update in source file
    assert!(
        assert_has_text_edit(
            doc_changes_anchor,
            "source.pn",
            &format!("[{note_name}#{new_anchor_name}]")
        ),
        "Anchor usage update failed"
    );

    println!("✅ Multiline content rename test passed");
}
