mod common;

use common::*;
use tower_lsp::lsp_types::{SemanticTokensResult, SemanticTokensRangeResult, Position, Range};

#[tokio::test]
async fn test_semantic_tokens_full() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "test.pn",
        "Normal text [wikilink] more text\n{@anchor section1}\n{@task status=todo due=2024-12-31}\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(
            uri.clone(),
            "Normal text [wikilink] more text\n{@anchor section1}\n{@task status=todo due=2024-12-31}\n"
                .to_string(),
        )
        .await;

    let result = client.semantic_tokens(uri).await;

    assert!(result.is_some(), "No result in semantic tokens");
    
    match result.unwrap() {
        SemanticTokensResult::Tokens(tokens) => {
            // Should have tokens for wikilink, anchor, task, etc.
            assert!(!tokens.data.is_empty(), "Semantic tokens data should not be empty");
        }
        SemanticTokensResult::Partial(_) => {
            panic!("Unexpected partial result");
        }
    }

    println!("✅ Semantic tokens full test passed");
}

#[tokio::test]
async fn test_semantic_tokens_range() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "test.pn",
        "Line 1 [link1]\nLine 2 [link2]\nLine 3 [link3]\nLine 4 [link4]\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(
            uri.clone(),
            "Line 1 [link1]\nLine 2 [link2]\nLine 3 [link3]\nLine 4 [link4]\n".to_string(),
        )
        .await;

    // Request tokens for lines 1-2 only
    let range = Range {
        start: Position { line: 1, character: 0 },
        end: Position { line: 2, character: 100 },
    };
    
    let result = client.semantic_tokens_range(uri, range).await;

    assert!(result.is_some(), "No result in semantic tokens range");
    
    match result.unwrap() {
        SemanticTokensRangeResult::Tokens(tokens) => {
            assert!(
                !tokens.data.is_empty(),
                "Should have tokens for the specified range"
            );
        }
        SemanticTokensRangeResult::Partial(_) => {
            panic!("Unexpected partial result");
        }
    }

    println!("✅ Semantic tokens range test passed");
}

#[tokio::test]
async fn test_semantic_tokens_empty_file() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("empty.pn", "");

    let mut client = InProcessLspClient::new(&workspace).await;

    let uri = workspace.get_uri("empty.pn");
    client.did_open(uri.clone(), "".to_string()).await;

    let result = client.semantic_tokens(uri).await;

    assert!(result.is_some(), "No result");

    match result.unwrap() {
        SemanticTokensResult::Tokens(tokens) => {
            // Empty file should have empty or minimal tokens
            assert_eq!(tokens.data.len(), 0, "Empty file should have no tokens");
        }
        SemanticTokensResult::Partial(_) => {
            panic!("Unexpected partial result");
        }
    }

    println!("✅ Semantic tokens empty file test passed");
}
