use serde_json::Value;

/// Assert that documentChanges contains a text edit with expected content
pub fn assert_has_text_edit(changes: &Value, file_name: &str, expected_text: &str) -> bool {
    if let Some(array) = changes.as_array() {
        for change in array {
            if let Some(text_doc) = change.get("textDocument") {
                if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                    let decoded_uri =
                        urlencoding::decode(uri).unwrap_or(std::borrow::Cow::Borrowed(uri));
                    if decoded_uri.contains(file_name) {
                        if let Some(edits) = change.get("edits").and_then(|v| v.as_array()) {
                            for edit in edits {
                                if let Some(new_text) = edit.get("newText").and_then(|v| v.as_str())
                                {
                                    if new_text == expected_text {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Assert that documentChanges contains a file rename operation
pub fn assert_has_file_rename(changes: &Value, old_name: &str, new_name: &str) -> bool {
    if let Some(array) = changes.as_array() {
        for change in array {
            if change.get("kind").and_then(|v| v.as_str()) == Some("rename") {
                let old_uri_raw = change.get("oldUri").and_then(|v| v.as_str()).unwrap_or("");
                let new_uri_raw = change.get("newUri").and_then(|v| v.as_str()).unwrap_or("");

                let old_uri = urlencoding::decode(old_uri_raw)
                    .unwrap_or(std::borrow::Cow::Borrowed(old_uri_raw));
                let new_uri = urlencoding::decode(new_uri_raw)
                    .unwrap_or(std::borrow::Cow::Borrowed(new_uri_raw));

                if old_uri.contains(old_name) && new_uri.contains(new_name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Assert that an edit preserves an anchor
pub fn assert_anchor_preserved(edit_text: &str, anchor: &str) -> bool {
    let expected = format!("#{}", anchor);
    edit_text.contains(&expected)
}

/// Assert that response has error with specific message pattern
pub fn assert_error_contains(response: &Value, pattern: &str) -> bool {
    if let Some(error) = response.get("error") {
        if let Some(message) = error.get("message").and_then(|v| v.as_str()) {
            return message.contains(pattern);
        }
    }
    false
}

/// Assert that capabilities include specific capability
pub fn assert_has_capability(init_response: &Value, capability_path: &[&str]) -> bool {
    let mut current = &init_response["result"]["capabilities"];
    for key in capability_path {
        current = &current[key];
        if current.is_null() {
            return false;
        }
    }
    true
}

/// Assert location points to specific file and position
pub fn assert_location(location: &Value, file_name: &str, line: u32, character: u32) -> bool {
    if let Some(uri) = location.get("uri").and_then(|v| v.as_str()) {
        if !uri.contains(file_name) {
            return false;
        }
    } else {
        return false;
    }

    if let Some(range) = location.get("range") {
        let start = &range["start"];
        if start["line"].as_u64() == Some(line as u64)
            && start["character"].as_u64() == Some(character as u64)
        {
            return true;
        }
    }
    false
}
