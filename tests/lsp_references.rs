mod common;

use common::*;

#[tokio::test]
async fn test_find_references_single_file() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "source.pn",
        "First [target]\nSecond [target]\nThird [target]\n",
    );
    workspace.create_file("target.pn", "Target content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let target_uri = workspace.get_uri("target.pn");
    client
        .did_open(target_uri.clone(), "Target content\n".to_string())
        .await;

    // Find references to target.pn
    let response = client.references(target_uri.clone(), 0, 0).await;

    assert!(response.is_some(), "No result in references");
    let refs = response.unwrap();

    // Should find 3 references
    assert_eq!(refs.len(), 3, "Expected 3 references");

    println!("✅ Find references single file test passed");
}

#[tokio::test]
async fn test_find_references_multiple_files() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("a.pn", "Link to [target]\n");
    workspace.create_file("b.pn", "Also [target] here\n");
    workspace.create_file("c.pn", "And [target] again\nTwice [target]\n");
    workspace.create_file("target.pn", "Target content\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let target_uri = workspace.get_uri("target.pn");
    client
        .did_open(target_uri.clone(), "Target content\n".to_string())
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let response = client.references(target_uri.clone(), 0, 0).await;

    assert!(response.is_some(), "No result in references");
    let refs = response.unwrap();

    // Should find 4 references total (1 in a, 1 in b, 2 in c)
    assert!(
        refs.len() >= 4,
        "Expected at least 4 references, got {}",
        refs.len()
    );

    println!("✅ Find references multiple files test passed");
}

#[tokio::test]
async fn test_find_references_with_anchors() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "source.pn",
        "Link [target#section1]\nAlso [target#section2]\nJust [target]\n",
    );
    workspace.create_file("target.pn", "{@anchor section1}\n{@anchor section2}\n");

    let mut client = InProcessLspClient::new(&workspace).await;

    let target_uri = workspace.get_uri("target.pn");
    client
        .did_open(
            target_uri.clone(),
            "{@anchor section1}\n{@anchor section2}\n".to_string(),
        )
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let response = client.references(target_uri.clone(), 0, 0).await;

    assert!(response.is_some(), "No result in references");
    let refs = response.unwrap();

    // All 3 links point to target.pn
    assert!(refs.len() >= 3, "Expected at least 3 references");

    println!("✅ Find references with anchors test passed");
}
