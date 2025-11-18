mod common;

use common::*;

#[tokio::test]
async fn test_goto_definition_basic_wikilink() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("source.pn", "Check this [target]\n");
    workspace.create_file("target.pn", "Target content\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "Check this [target]\n".to_string())
        .await;

    // Position inside [target] at line 0, char 13
    let response = client.definition(source_uri, 0, 13).await;

    assert!(response.get("result").is_some(), "No result in definition");
    let result = &response["result"];
    assert!(result["uri"].is_string(), "No uri in definition");
    assert!(result["uri"].as_str().unwrap().contains("target.pn"));

    println!("✅ Basic goto definition test passed");
}

#[tokio::test]
async fn test_goto_definition_with_anchor() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("source.pn", "See [target#section2]\n");
    workspace.create_file(
        "target.pn",
        "Line 1\n#section1\nLine 2\n#section2\nLine 3\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    // Wait for workspace scan to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "See [target#section2]\n".to_string())
        .await;

    // Position inside [target#section2]
    let response = client.definition(source_uri, 0, 8).await;

    assert!(response.get("result").is_some(), "No result in definition");
    let result = &response["result"];
    assert!(result["uri"].as_str().unwrap().contains("target.pn"));

    // Should point to the line with section2 anchor
    // Line 0: Line 1
    // Line 1: #section1
    // Line 2: Line 2
    // Line 3: #section2
    // Line 4: Line 3
    let range = &result["range"];
    let start_line = range["start"]["line"].as_u64().unwrap();
    // If workspace scan completed, should point to line 3
    // Otherwise, it points to line 0 (start of file)
    assert_eq!(start_line, 3);
    //if start_line == 3 {
    //    println!("✅ Goto definition with anchor found correctly (line 3)");
    //} else {
    //    println!("⚠️  Goto definition returned start of file (line 0) - anchor not found yet");
    //    println!("    This might happen if workspace scan is still in progress");
    //}
    println!("✅ Goto definition with anchor test passed");
}

#[tokio::test]
async fn test_goto_definition_nonexistent_note() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("source.pn", "Link to [nonexistent]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "Link to [nonexistent]\n".to_string())
        .await;

    let response = client.definition(source_uri, 0, 12).await;

    // Should still return a result (file will be created on navigation)
    assert!(response.get("result").is_some());

    println!("✅ Goto definition for nonexistent note test passed");
}
