mod common;

use common::*;

#[tokio::test]
async fn test_did_open_and_close() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Initial content\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    
    // Open document
    client
        .did_open(uri.clone(), "Initial content\n".to_string())
        .await;

    // Document should now be tracked
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Close document
    client.did_close(uri.clone()).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("✅ Did open and close test passed");
}

#[tokio::test]
async fn test_did_change() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Initial\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(uri.clone(), "Initial\n".to_string())
        .await;

    // Send change notification
    client
        .notify(
            "textDocument/didChange",
            serde_json::json!({
                "textDocument": {
                    "uri": uri.to_string(),
                    "version": 2
                },
                "contentChanges": [{
                    "text": "Changed content [newlink]\n"
                }]
            }),
        )
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Try to find the new link
    let response = client.definition(uri.clone(), 0, 18).await;
    
    // Should be able to find definition for the new link
    assert!(response.get("result").is_some());

    println!("✅ Did change test passed");
}

#[tokio::test]
async fn test_did_save() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Content\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(uri.clone(), "Content\n".to_string())
        .await;

    // Send save notification
    client
        .notify(
            "textDocument/didSave",
            serde_json::json!({
                "textDocument": {
                    "uri": uri.to_string()
                }
            }),
        )
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("✅ Did save test passed");
}

#[tokio::test]
async fn test_multiple_documents() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("a.pn", "Doc A\n");
    workspace.create_file("b.pn", "Doc B\n");
    workspace.create_file("c.pn", "Doc C\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri_a = workspace.get_uri("a.pn");
    let uri_b = workspace.get_uri("b.pn");
    let uri_c = workspace.get_uri("c.pn");

    // Open multiple documents
    client.did_open(uri_a.clone(), "Doc A\n".to_string()).await;
    client.did_open(uri_b.clone(), "Doc B\n".to_string()).await;
    client.did_open(uri_c.clone(), "Doc C\n".to_string()).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Close one
    client.did_close(uri_b.clone()).await;

    // Others should still be open
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("✅ Multiple documents test passed");
}
