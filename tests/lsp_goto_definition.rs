mod common;

use common::*;
use tower_lsp::lsp_types::GotoDefinitionResponse;

#[tokio::test]
async fn test_goto_definition_basic_wikilink() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("source.pn", "Check this [target]\n");
    workspace.create_file("target.pn", "Target content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "Check this [target]\n".to_string())
        .await;

    // Position inside [target] at line 0, char 13
    let response = client.definition(source_uri, 0, 13).await;

    assert!(response.is_some(), "No result in definition");
    match response.unwrap() {
        GotoDefinitionResponse::Scalar(location) => {
            assert!(location.uri.as_str().contains("target.pn"));
        }
        GotoDefinitionResponse::Array(locations) => {
            assert!(!locations.is_empty());
            assert!(locations[0].uri.as_str().contains("target.pn"));
        }
        GotoDefinitionResponse::Link(_) => panic!("Unexpected Link response"),
    }

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

    let mut client = InProcessLspClient::new(&workspace).await;

    // Wait for workspace scan to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "See [target#section2]\n".to_string())
        .await;

    // Position inside [target#section2]
    let response = client.definition(source_uri, 0, 8).await;

    assert!(response.is_some(), "No result in definition");
    
    let location = match response.unwrap() {
        GotoDefinitionResponse::Scalar(loc) => loc,
        GotoDefinitionResponse::Array(locs) => locs[0].clone(),
        GotoDefinitionResponse::Link(_) => panic!("Unexpected Link response"),
    };
    
    assert!(location.uri.as_str().contains("target.pn"));

    // Should point to the line with section2 anchor
    // Line 0: Line 1
    // Line 1: #section1
    // Line 2: Line 2
    // Line 3: #section2
    // Line 4: Line 3
    let start_line = location.range.start.line;
    // If workspace scan completed, should point to line 3
    assert_eq!(start_line, 3);
    
    println!("✅ Goto definition with anchor test passed");
}

#[tokio::test]
async fn test_goto_definition_nonexistent_note() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("source.pn", "Link to [nonexistent]\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "Link to [nonexistent]\n".to_string())
        .await;

    let response = client.definition(source_uri, 0, 12).await;

    // Should still return a result (file will be created on navigation)
    assert!(response.is_some());

    println!("✅ Goto definition for nonexistent note test passed");
}
