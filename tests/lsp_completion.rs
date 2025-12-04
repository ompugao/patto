mod common;

use common::*;

#[tokio::test]
async fn test_completion_note_names() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("note_one.pn", "Content 1\n");
    workspace.create_file("note_two.pn", "Content 2\n");
    workspace.create_file("another_note.pn", "Content 3\n");
    workspace.create_file("source.pn", "Link [no\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "Link [no\n".to_string())
        .await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Position after "no" inside [no
    let response = client.completion(source_uri, 0, 8).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(!items.is_empty(), "No completion items");

        // Should suggest note_one and note_two (fuzzy match "no")
        let labels: Vec<String> = items
            .iter()
            .filter_map(|item| item["label"].as_str().map(String::from))
            .collect();

        assert!(
            labels.iter().any(|l| l.contains("note")),
            "Should suggest notes matching 'no'"
        );
    }

    println!("✅ Note name completion test passed");
}

#[tokio::test]
async fn test_completion_anchors() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("target.pn", "#intro\nContent\n#summary\n#conclusion\n");
    workspace.create_file("source.pn", "See [target#\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    // Wait for workspace scan to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "See [target#\n".to_string())
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Position after # in [target#
    let response = client.completion(source_uri, 0, 12).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(
            !items.is_empty(),
            "⚠️  No anchor completions (workspace scan may still be in progress)"
        );
        let labels: Vec<String> = items
            .iter()
            .filter_map(|item| item["label"].as_str().map(String::from))
            .collect();

        // Should suggest anchors with # prefix
        assert!(
            labels
                .iter()
                .any(|l| l.contains("intro") || l.contains("#intro")),
            "Should suggest intro anchor, got: {:?}",
            labels
        );
        println!("Found anchor completions: {:?}", labels);
    }

    println!("✅ Anchor completion test passed");
}

#[tokio::test]
async fn test_completion_code_command() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "@code\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client.did_open(uri.clone(), "@code\n".to_string()).await;

    // Position after @code
    let response = client.completion(uri, 0, 5).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(!items.is_empty(), "No code completions");

        let first_item = &items[0];
        assert_eq!(first_item["label"].as_str(), Some("@code"));
    }

    println!("✅ Code command completion test passed");
}

#[tokio::test]
async fn test_completion_math_command() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "@math\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client.did_open(uri.clone(), "@math\n".to_string()).await;

    let response = client.completion(uri, 0, 5).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(!items.is_empty(), "No math completions");
        assert_eq!(items[0]["label"].as_str(), Some("@math"));
    }

    println!("✅ Math command completion test passed");
}

#[tokio::test]
async fn test_completion_task_property() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "@task\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client.did_open(uri.clone(), "@task\n".to_string()).await;

    let response = client.completion(uri, 0, 5).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(!items.is_empty(), "No task completions");
        assert_eq!(items[0]["label"].as_str(), Some("@task"));

        // Check that it includes snippet with status and due date
        if let Some(text_edit) = items[0]["textEdit"].as_object() {
            let new_text = text_edit["newText"].as_str().unwrap();
            assert!(new_text.contains("status"), "Should include status field");
            assert!(new_text.contains("due"), "Should include due field");
        }
    }

    println!("✅ Task property completion test passed");
}

#[tokio::test]
async fn test_completion_quote_command() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "@quote\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client.did_open(uri.clone(), "@quote\n".to_string()).await;

    let response = client.completion(uri, 0, 6).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(!items.is_empty(), "No quote completions");
        assert_eq!(items[0]["label"].as_str(), Some("@quote"));
    }

    println!("✅ Quote command completion test passed");
}

#[tokio::test]
async fn test_completion_img_command() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "@img\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client.did_open(uri.clone(), "@img\n".to_string()).await;

    let response = client.completion(uri, 0, 4).await;

    assert!(response.get("result").is_some(), "No result in completion");
    let result = &response["result"];

    if result.is_array() {
        let items = result.as_array().unwrap();
        assert!(!items.is_empty(), "No img completions");
        assert_eq!(items[0]["label"].as_str(), Some("@img"));
    }

    println!("✅ Image command completion test passed");
}
