mod common;

use common::*;

#[tokio::test]
async fn test_aggregate_tasks_empty() {
    let workspace = TestWorkspace::new();
    let mut client = LspTestClient::new(&workspace).await;

    client.initialize().await;
    client.initialized().await;

    let response = client.aggregate_tasks().await;

    // Should return empty array when no tasks
    assert!(
        response.get("result").is_some(),
        "No result in aggregate_tasks"
    );
    let result = &response["result"];
    assert!(
        result.is_array() || result.is_null(),
        "Result should be array or null"
    );

    println!("✅ Aggregate tasks empty test passed");
}

#[tokio::test]
async fn test_aggregate_tasks_with_tasks() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "todo.pn",
        "Task 1 {@task status=todo due=2024-12-31}\nTask 2 {@task status=doing due=2024-12-25}\n",
    );
    workspace.create_file("done.pn", "Completed {@task status=done due=2024-12-20}\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    // Wait for workspace scan
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let response = client.aggregate_tasks().await;

    assert!(
        response.get("result").is_some(),
        "No result in aggregate_tasks"
    );
    let result = &response["result"];
    assert!(result.is_array(), "Result should be array");

    let tasks = result.as_array().unwrap();
    assert!(
        tasks.len() >= 2,
        "Should have at least 2 tasks, got {}",
        tasks.len()
    );

    // Check task structure - based on TaskInformation struct
    // Fields: location, text, message, due
    for (i, task) in tasks.iter().enumerate() {
        assert!(task.is_object(), "Task {} should be an object", i);
        assert!(
            task["location"].is_object(),
            "Task {} should have location object",
            i
        );
        assert!(
            task["location"]["uri"].is_string(),
            "Task {} location should have uri",
            i
        );
        assert!(
            task["location"]["range"].is_object(),
            "Task {} location should have range",
            i
        );
        assert!(
            task["text"].is_string(),
            "Task {} should have text string",
            i
        );
        assert!(
            task["message"].is_string(),
            "Task {} should have message string",
            i
        );
        assert!(
            task["due"].is_object() || task["due"].is_string(),
            "Task {} should have due field",
            i
        );
    }

    // Verify at least one task has expected content
    let task_texts: Vec<&str> = tasks.iter().filter_map(|t| t["text"].as_str()).collect();
    assert!(
        task_texts
            .iter()
            .any(|t| t.contains("Task 1") || t.contains("Task 2") || t.contains("Completed")),
        "Should have tasks with expected text, got: {:?}",
        task_texts
    );

    println!("✅ Aggregate tasks with tasks test passed");
}

#[tokio::test]
async fn test_aggregate_tasks_sorting() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "tasks.pn",
        "Future {@task status=todo due=2025-12-31}\nSoon {@task status=todo due=2024-12-25}\nPast {@task status=todo due=2024-01-01}\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let response = client.aggregate_tasks().await;

    assert!(response.get("result").is_some(), "No result");
    let result = &response["result"];
    assert!(result.is_array(), "Result should be array");

    let tasks = result.as_array().unwrap();
    assert_eq!(
        tasks.len(),
        3,
        "Should have exactly 3 tasks, got {}",
        tasks.len()
    );

    // Extract due dates from tasks
    let due_dates: Vec<String> = tasks
        .iter()
        .filter_map(|task| task["due"]["Date"].as_str().map(String::from))
        .collect();

    assert_eq!(due_dates.len(), 3, "All tasks should have due dates");

    // Tasks should be sorted by due date (earliest first)
    // Verify ordering: 2024-01-01, 2024-12-25, 2025-12-31
    assert_eq!(
        due_dates[0], "2024-01-01",
        "First task should be Past (2024-01-01)"
    );
    assert_eq!(
        due_dates[1], "2024-12-25",
        "Second task should be Soon (2024-12-25)"
    );
    assert_eq!(
        due_dates[2], "2025-12-31",
        "Third task should be Future (2025-12-31)"
    );

    println!("✅ Aggregate tasks sorting test passed");
}

#[tokio::test]
async fn test_two_hop_links_basic() {
    let mut workspace = TestWorkspace::new();
    // Create a link graph where:
    // - source -> target
    // - other -> target (both source and other link to target)
    // This creates a two-hop connection: source -> target <- other
    workspace.create_file("source.pn", "Link to [target]\n");
    workspace.create_file("target.pn", "Target content\n");
    workspace.create_file("other.pn", "Also links to [target]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let source_uri = workspace.get_uri("source.pn");
    client
        .did_open(source_uri.clone(), "Link to [target]\n".to_string())
        .await;

    // Wait for workspace scan and graph building
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let response = client.two_hop_links(source_uri.clone()).await;

    // Should return two-hop connections
    assert!(
        response.get("result").is_some(),
        "No result in two_hop_links"
    );
    let result = &response["result"];

    // Result should be an array of [target_url, [connected_urls]]
    if result.is_array() {
        let links = result.as_array().unwrap();
        println!("Two-hop links result: {:?}", links);

        // Should find that both source and other link to target
        // So the result should include target with other.pn in the connected list
        assert!(!links.is_empty());
        let first_link = &links[0];
        if first_link.is_array() {
            let pair = first_link.as_array().unwrap();
            if pair.len() == 2 {
                let target_url = &pair[0];
                let connected = &pair[1];

                // target_url should contain "target.pn"
                if target_url.is_string() {
                    assert!(
                        target_url.as_str().unwrap().contains("target.pn"),
                        "First element should be target.pn"
                    );
                }

                // connected should be an array containing other.pn
                if connected.is_array() {
                    let connected_files = connected.as_array().unwrap();
                    println!("Connected files: {:?}", connected_files);
                    assert!(
                        !connected_files.is_empty(),
                        "Should have connections (other.pn links to target too)"
                    );

                    // Verify that other.pn is in the list and source.pn is NOT
                    let urls_str = connected_files
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>();

                    assert!(
                        urls_str.iter().any(|u| u.contains("other.pn")),
                        "Should include other.pn"
                    );
                    assert!(
                        !urls_str.iter().any(|u| u.contains("source.pn")),
                        "Should NOT include source.pn (it's the query file)"
                    );
                }
            }
        }
    }

    println!("✅ Two-hop links basic test passed");
}

#[tokio::test]
async fn test_two_hop_links_no_connections() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("isolated.pn", "No links here\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("isolated.pn");
    client
        .did_open(uri.clone(), "No links here\n".to_string())
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let response = client.two_hop_links(uri).await;

    assert!(response.get("result").is_some(), "No result");
    let result = &response["result"];

    if result.is_array() {
        let links = result.as_array().unwrap();
        assert_eq!(links.len(), 0, "Should have no two-hop links");
    }

    println!("✅ Two-hop links no connections test passed");
}
