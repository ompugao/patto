mod common;

use common::*;
use serde_json::{json, Value};

/// Helper function to apply semantic token edits to a base token array
fn apply_semantic_token_edits(base_data: &[Value], edits: &[Value]) -> Vec<Value> {
    let mut result = base_data.to_vec();

    // Apply edits in order (they should be sorted by start position)
    for edit in edits {
        let start = edit["start"].as_u64().expect("start should be u64") as usize;
        let delete_count = edit["deleteCount"]
            .as_u64()
            .expect("deleteCount should be u64") as usize;

        // Remove deleted elements
        if delete_count > 0 {
            result.drain(start..start + delete_count);
        }

        // Insert new data if present
        if let Some(data) = edit.get("data") {
            let data_array = data.as_array().expect("data should be array");
            for (i, item) in data_array.iter().enumerate() {
                result.insert(start + i, item.clone());
            }
        }
    }

    result
}

#[tokio::test]
async fn test_semantic_tokens_full_returns_result_id() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "test.pn",
        "Normal text [wikilink] more text\n{@anchor section1}\n",
    );

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(
            uri.clone(),
            "Normal text [wikilink] more text\n{@anchor section1}\n".to_string(),
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

    // Verify result_id is present
    assert!(
        result.get("resultId").is_some(),
        "result_id should be present in full tokens response"
    );

    // Verify data is present and valid
    let data = result["data"].as_array().expect("data should be an array");
    assert!(!data.is_empty(), "Should have tokens");

    // Verify token structure - each token is 5 u32 values
    assert_eq!(data.len() % 5, 0, "Token count should be multiple of 5");

    // Verify token values are valid u32
    for (i, val) in data.iter().enumerate() {
        assert!(
            val.is_u64() || val.is_i64(),
            "Token value at index {} should be numeric",
            i
        );
    }

    println!("✅ Full tokens with result_id test passed");
}

#[tokio::test]
async fn test_semantic_tokens_delta_with_valid_result_id() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Line 1 [link1]\nLine 2 [link2]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(uri.clone(), "Line 1 [link1]\nLine 2 [link2]\n".to_string())
        .await;

    // Get initial full tokens with result_id
    let full_response = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    let result_id = full_response["result"]["resultId"]
        .as_str()
        .expect("result_id should be a string");

    let original_data = full_response["result"]["data"]
        .as_array()
        .expect("data should be an array")
        .clone();

    // Simulate document change by closing and reopening with new content
    client
        .did_open(
            uri.clone(),
            "Line 1 [link1]\nLine 2 [link2]\nLine 3 [link3]\n".to_string(),
        )
        .await;

    // Get the expected new full tokens
    let expected_full_response = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    let expected_new_data = expected_full_response["result"]["data"]
        .as_array()
        .expect("expected data should be an array")
        .clone();

    // Now request delta with previous result_id
    client
        .did_open(uri.clone(), "Line 1 [link1]\nLine 2 [link2]\n".to_string())
        .await;

    let _reset_full = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    client
        .did_open(
            uri.clone(),
            "Line 1 [link1]\nLine 2 [link2]\nLine 3 [link3]\n".to_string(),
        )
        .await;

    let delta_response = client
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "previousResultId": result_id
            }),
        )
        .await;

    assert!(
        delta_response.get("result").is_some(),
        "No result in delta response"
    );

    let delta_result = &delta_response["result"];

    // Should have a new result_id
    assert!(
        delta_result.get("resultId").is_some(),
        "Delta response should have result_id"
    );

    // Verify delta structure and apply edits
    if let Some(edits) = delta_result.get("edits") {
        // Delta response - verify edits structure
        let edits_array = edits.as_array().expect("edits should be an array");

        // Should have at least one edit since we added content
        assert!(
            !edits_array.is_empty(),
            "Should have edits when content changed"
        );

        // Verify each edit has required fields and valid values
        for edit in edits_array {
            assert!(
                edit.get("start").is_some(),
                "Edit should have 'start' field"
            );
            assert!(
                edit.get("deleteCount").is_some(),
                "Edit should have 'deleteCount' field"
            );

            let start = edit["start"].as_u64().expect("start should be a number");
            let delete_count = edit["deleteCount"]
                .as_u64()
                .expect("deleteCount should be a number");

            // Validate that start and deleteCount are multiples of 5 (semantic token size)
            assert_eq!(
                start % 5,
                0,
                "start should be a multiple of 5 (semantic token size)"
            );
            assert_eq!(
                delete_count % 5,
                0,
                "deleteCount should be a multiple of 5 (semantic token size)"
            );

            // Verify start is within bounds
            assert!(
                (start as usize) <= original_data.len(),
                "start should be within original data bounds"
            );

            // Verify delete_count doesn't exceed available data
            assert!(
                (start as usize + delete_count as usize) <= original_data.len(),
                "start + deleteCount should not exceed original data length"
            );

            // If data is present, verify it's valid
            if let Some(data) = edit.get("data") {
                let data_array = data.as_array().expect("edit data should be array");
                assert_eq!(
                    data_array.len() % 5,
                    0,
                    "edit data length should be multiple of 5"
                );

                // Verify all values are numeric
                for val in data_array {
                    assert!(
                        val.is_u64() || val.is_i64(),
                        "edit data values should be numeric"
                    );
                }
            }
        }

        // Apply the delta edits to original data and verify result matches expected
        let reconstructed_data = apply_semantic_token_edits(&original_data, edits_array);

        assert_eq!(
            reconstructed_data, expected_new_data,
            "Applying delta edits to original data should produce the new full tokens"
        );

        println!(
            "✅ Delta edits validated and successfully applied: {} edit(s)",
            edits_array.len()
        );
    } else if let Some(data) = delta_result.get("data") {
        // Full fallback response
        let data_array = data.as_array().expect("data should be an array");

        // Should match expected new data
        assert_eq!(
            *data_array, expected_new_data,
            "Full fallback should match expected new tokens"
        );

        println!("✅ Full fallback validated (cache was invalidated)");
    } else {
        panic!("Delta response should have either 'edits' or 'data'");
    }

    println!("✅ Delta with valid result_id test passed");
}

#[tokio::test]
async fn test_semantic_tokens_delta_with_invalid_result_id() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Line 1 [link1]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(uri.clone(), "Line 1 [link1]\n".to_string())
        .await;

    // Request delta with invalid/non-existent result_id
    let delta_response = client
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "previousResultId": "invalid-result-id-12345"
            }),
        )
        .await;

    assert!(
        delta_response.get("result").is_some(),
        "No result in delta response"
    );

    let delta_result = &delta_response["result"];

    // Should return full tokens (not delta) when result_id doesn't match
    assert!(
        delta_result.get("data").is_some(),
        "Should return full tokens when result_id is invalid"
    );

    // Should NOT have edits
    assert!(
        delta_result.get("edits").is_none(),
        "Should not have edits when returning full tokens"
    );

    // Verify data structure
    let data = delta_result["data"]
        .as_array()
        .expect("data should be an array");

    // Should have tokens for the link
    assert!(!data.is_empty(), "Should have tokens for the content");

    // Verify data is in correct format (multiples of 5)
    assert_eq!(
        data.len() % 5,
        0,
        "Token data should be in groups of 5 (deltaLine, deltaStart, length, tokenType, modifiers)"
    );

    // Verify all token values are valid
    for (i, val) in data.iter().enumerate() {
        assert!(
            val.is_u64() || val.is_i64(),
            "Token value at index {} should be numeric",
            i
        );
    }

    assert!(
        delta_result.get("resultId").is_some(),
        "Should return new result_id"
    );

    println!("✅ Delta with invalid result_id test passed");
}

#[tokio::test]
async fn test_semantic_tokens_delta_after_cache_invalidation() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Line 1 [link1]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(uri.clone(), "Line 1 [link1]\n".to_string())
        .await;

    // Get initial full tokens
    let full_response = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    let result_id = full_response["result"]["resultId"]
        .as_str()
        .expect("result_id should be a string");

    // Change document (this invalidates cache)
    client
        .did_open(uri.clone(), "Line 1 [link1]\nLine 2 [link2]\n".to_string())
        .await;

    // Request delta - cache was invalidated, so should return full tokens
    let delta_response = client
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "previousResultId": result_id
            }),
        )
        .await;

    let delta_result = &delta_response["result"];

    // Should return full tokens because cache was invalidated
    assert!(
        delta_result.get("data").is_some(),
        "Should return full tokens after cache invalidation"
    );

    let data = delta_result["data"]
        .as_array()
        .expect("data should be array");

    // Verify token structure
    assert_eq!(data.len() % 5, 0, "Should have valid token structure");
    assert!(!data.is_empty(), "Should have tokens for new content");

    println!("✅ Delta after cache invalidation test passed");
}

#[tokio::test]
async fn test_semantic_tokens_delta_no_changes() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("test.pn", "Static content [link]\n");

    let mut client = LspTestClient::new(&workspace).await;
    client.initialize().await;
    client.initialized().await;

    let uri = workspace.get_uri("test.pn");
    client
        .did_open(uri.clone(), "Static content [link]\n".to_string())
        .await;

    // Get initial tokens
    let full_response1 = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    let result_id1 = full_response1["result"]["resultId"]
        .as_str()
        .expect("result_id should be a string");

    let data1 = full_response1["result"]["data"]
        .as_array()
        .expect("data should be an array")
        .clone();

    // Get tokens again without changes (simulating re-request)
    let full_response2 = client
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri.to_string() }
            }),
        )
        .await;

    let result_id2 = full_response2["result"]["resultId"]
        .as_str()
        .expect("result_id should be a string");

    let data2 = full_response2["result"]["data"]
        .as_array()
        .expect("data should be an array");

    // Result IDs should be the same if content hasn't changed
    assert_eq!(
        result_id1, result_id2,
        "Result IDs should match when content is unchanged"
    );

    // Token data should be identical - verify element by element
    assert_eq!(
        data1.len(),
        data2.len(),
        "Token arrays should have same length"
    );
    for (i, (v1, v2)) in data1.iter().zip(data2.iter()).enumerate() {
        assert_eq!(v1, v2, "Token value at index {} should be identical", i);
    }

    // Verify we have tokens for the link
    assert!(!data1.is_empty(), "Should have tokens for the link");

    // Verify token structure
    assert_eq!(data1.len() % 5, 0, "Should have valid token structure");

    println!("✅ Delta with no changes test passed");
}

#[tokio::test]
async fn test_semantic_tokens_range_still_works() {
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

    // Verify token structure
    assert_eq!(
        data.len() % 5,
        0,
        "Range tokens should have valid structure"
    );

    // Verify all values are numeric
    for (i, val) in data.iter().enumerate() {
        assert!(
            val.is_u64() || val.is_i64(),
            "Range token value at index {} should be numeric",
            i
        );
    }

    println!("✅ Range tokens still work test passed");
}
