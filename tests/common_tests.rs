//! Tests for the common test infrastructure
//! This file runs these tests only once instead of duplicating them in every test binary

mod common;

use common::TestWorkspace;

#[test]
fn test_workspace_creation() {
    let mut workspace = TestWorkspace::new();
    let path = workspace.create_file("test.pn", "content");

    assert!(path.exists());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "content");

    let uri = workspace.get_uri("test.pn");
    assert!(uri.as_str().contains("test.pn"));
}
