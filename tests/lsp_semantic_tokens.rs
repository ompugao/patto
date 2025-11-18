mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn test_semantic_tokens_full() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "test.pn",
        "Normal text [wikilink] more text\n{@anchor section1}\n{@task status=todo due=2024-12-31}\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(
            uri.clone(),
            "Normal text [wikilink] more text\n{@anchor section1}\n{@task status=todo due=2024-12-31}\n"
                .to_string(),
        )
        .await;

    let response = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    assert!(
        response.get("result").is_some(),
        "No result in semantic tokens"
    );
    let result = &response["result"];
    assert!(
        result["data"].is_array(),
        "No data array in semantic tokens"
    );

    let data = result["data"].as_array().unwrap();
    // Should have tokens for wikilink, anchor, task, etc.
    assert!(!data.is_empty(), "Semantic tokens data should not be empty");

    println!("✅ Semantic tokens full test passed");
}

#[tokio::test]
async fn test_semantic_tokens_range() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "test.pn",
        "Line 1 [link1]\nLine 2 [link2]\nLine 3 [link3]\nLine 4 [link4]\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(
            uri.clone(),
            "Line 1 [link1]\nLine 2 [link2]\nLine 3 [link3]\nLine 4 [link4]\n".to_string(),
        )
        .await;

    // Request tokens for lines 1-2 only
    let response = client
        .request(
            "textDocument/semanticTokens/range",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 2, "character": 100 }
                }
            }),
        )
        .await;

    assert!(
        response.get("result").is_some(),
        "No result in semantic tokens range"
    );
    let result = &response["result"];
    assert!(
        result["data"].is_array(),
        "No data array in semantic tokens range"
    );

    let data = result["data"].as_array().unwrap();
    assert!(
        !data.is_empty(),
        "Should have tokens for the specified range"
    );

    println!("✅ Semantic tokens range test passed");
}

#[tokio::test]
async fn test_semantic_tokens_empty_file() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("empty.pn", "");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("empty.pn");
    client.did_open(uri.clone(), "".to_string()).await;

    let response = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    assert!(response.get("result").is_some(), "No result");
    let result = &response["result"];

    if result.is_object() && result["data"].is_array() {
        let data = result["data"].as_array().unwrap();
        // Empty file should have empty or minimal tokens
        assert_eq!(data.len(), 0, "Empty file should have no tokens");
    }

    println!("✅ Semantic tokens empty file test passed");
}
