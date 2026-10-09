//! Predicates over JSON-encoded LSP responses. They return `bool` so a test
//! can wrap them in `assert!` with its own message.

use std::borrow::Cow;

use serde_json::Value;

fn decoded(uri: &str) -> Cow<'_, str> {
    urlencoding::decode(uri).unwrap_or(Cow::Borrowed(uri))
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

/// `documentChanges` holds a text edit on `file_name` inserting `expected_text`.
pub fn assert_has_text_edit(changes: &Value, file_name: &str, expected_text: &str) -> bool {
    let Some(changes) = changes.as_array() else {
        return false;
    };
    changes
        .iter()
        .filter(|change| {
            change
                .get("textDocument")
                .and_then(|doc| str_at(doc, "uri"))
                .is_some_and(|uri| decoded(uri).contains(file_name))
        })
        .filter_map(|change| change.get("edits")?.as_array())
        .flatten()
        .any(|edit| str_at(edit, "newText") == Some(expected_text))
}

/// `documentChanges` holds a file rename from `old_name` to `new_name`.
pub fn assert_has_file_rename(changes: &Value, old_name: &str, new_name: &str) -> bool {
    let Some(changes) = changes.as_array() else {
        return false;
    };
    changes
        .iter()
        .filter(|change| str_at(change, "kind") == Some("rename"))
        .any(|change| {
            let old_uri = decoded(str_at(change, "oldUri").unwrap_or(""));
            let new_uri = decoded(str_at(change, "newUri").unwrap_or(""));
            old_uri.contains(old_name) && new_uri.contains(new_name)
        })
}

pub fn assert_anchor_preserved(edit_text: &str, anchor: &str) -> bool {
    edit_text.contains(&format!("#{}", anchor))
}

pub fn assert_error_contains(response: &Value, pattern: &str) -> bool {
    response
        .get("error")
        .and_then(|error| str_at(error, "message"))
        .is_some_and(|message| message.contains(pattern))
}

/// `result.capabilities` has a non-null value at `capability_path`.
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

/// `location` is in `file_name` and starts at `(line, character)`.
pub fn assert_location(location: &Value, file_name: &str, line: u32, character: u32) -> bool {
    let in_file = str_at(location, "uri").is_some_and(|uri| uri.contains(file_name));
    let start = &location["range"]["start"];
    in_file
        && start["line"].as_u64() == Some(line as u64)
        && start["character"].as_u64() == Some(character as u64)
}
