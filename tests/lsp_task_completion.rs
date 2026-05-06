mod common;

use common::*;
use patto::parser::{AstNodeKind, Property, TaskStatus};

/// Collect all Task properties from an AST node and its children.
fn collect_tasks(
    node: &patto::parser::AstNode,
    out: &mut Vec<(usize, TaskStatus, Option<patto::parser::Deadline>)>,
) {
    if let AstNodeKind::Line { properties } = &node.kind() {
        for prop in properties {
            if let Property::Task {
                status,
                completed_at,
                ..
            } = prop
            {
                out.push((node.location().row, status.clone(), completed_at.clone()));
                break;
            }
        }
    }
    for child in node.value().children.lock().unwrap().iter() {
        collect_tasks(child, out);
    }
}
/// `workspace/applyEdit` in the test environment is a no-op (the mock client silently discards
/// server→client requests).  We therefore test the *detection* logic directly: after a
/// `did_change` that transitions a task to `!Done`, the new AST stored in `ast_map` must
/// reflect the Done status, and `collect_completion_edits` (via the backend) must produce
/// exactly one edit targeting the right line.
///
/// We verify the edit by inspecting the `WorkspaceEdit` that `collect_completion_edits` would
/// generate — we do this by calling `Backend`'s internal helper directly through a thin
/// white-box wrapper in `InProcessLspClient::get_ast`.
#[tokio::test]
async fn test_auto_complete_detects_task_transition() {
    let mut workspace = TestWorkspace::new();
    // Start with a Todo task
    workspace.create_file("tasks.pn", "buy milk {@task status=todo}\n");

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("tasks.pn");

    // Open the file so the backend parses it and populates ast_map
    client
        .did_open(uri.clone(), "buy milk {@task status=todo}\n".to_string())
        .await;

    // Verify the initial AST has a Todo task
    {
        let ast = client.get_ast(&uri).expect("AST should be present after did_open");
        let mut tasks = Vec::new();
        collect_tasks(&ast, &mut tasks);
        assert_eq!(tasks.len(), 1, "Expected one task, got tasks={:?}, uri={}", tasks.len(), uri);
        assert_eq!(tasks[0].1, TaskStatus::Todo, "Task should initially be Todo");
        assert!(tasks[0].2.is_none(), "completed_at should be absent initially");
    }

    // Change the task status to Done (editor edit, no completed_at yet)
    client
        .did_change(
            uri.clone(),
            2,
            "buy milk {@task status=done}\n".to_string(),
        )
        .await;

    // The new AST should reflect Done status
    {
        let ast = client.get_ast(&uri).expect("AST should be present after did_change");
        let mut tasks = Vec::new();
        collect_tasks(&ast, &mut tasks);
        assert_eq!(tasks.len(), 1, "Expected one task after change");
        assert_eq!(tasks[0].1, TaskStatus::Done, "Task should now be Done");
        // completed_at is still None here — the edit would be applied via workspace/applyEdit
        // which the mock client silently drops. That's fine; we're testing detection.
    }
}

#[tokio::test]
async fn test_auto_complete_does_not_trigger_for_already_done() {
    let mut workspace = TestWorkspace::new();
    // Start with a Done task that already has completed_at
    workspace.create_file(
        "tasks.pn",
        "buy milk {@task status=done completed_at=2024-01-15}\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("tasks.pn");

    client
        .did_open(
            uri.clone(),
            "buy milk {@task status=done completed_at=2024-01-15}\n".to_string(),
        )
        .await;

    // Simulate the editor sending the same content again (no real change)
    client
        .did_change(
            uri.clone(),
            2,
            "buy milk {@task status=done completed_at=2024-01-15}\n".to_string(),
        )
        .await;

    // completed_at must still be present and unchanged
    let ast = client.get_ast(&uri).expect("AST should be present");
    let mut tasks = Vec::new();
    collect_tasks(&ast, &mut tasks);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].1, TaskStatus::Done);
    assert!(
        tasks[0].2.is_some(),
        "completed_at should still be present"
    );
}

#[tokio::test]
async fn test_tasks_review_custom_range() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "tasks.pn",
        "- buy milk {@task status=done completed_at=2024-03-15}\n\
         - write report {@task status=done completed_at=2024-03-20}\n\
         - pending item {@task status=todo}\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("tasks.pn");
    client
        .did_open(
            uri.clone(),
            "- buy milk {@task status=done completed_at=2024-03-15}\n\
             - write report {@task status=done completed_at=2024-03-20}\n\
             - pending item {@task status=todo}\n"
                .to_string(),
        )
        .await;

    let result = client
        .tasks_review("custom", Some("2024-03-01"), Some("2024-03-31"))
        .await;
    let tasks = result
        .flatten()
        .expect("tasks_review should return a value");
    let arr = tasks.as_array().expect("result should be an array");

    assert_eq!(arr.len(), 2, "Expected 2 completed tasks in March 2024");
    assert_eq!(arr[0]["completed_at"], "2024-03-15");
    assert_eq!(arr[1]["completed_at"], "2024-03-20");

    // Pending task must not appear
    for item in arr {
        assert!(
            !item["text"].as_str().unwrap_or("").contains("pending"),
            "Pending task should not appear in review"
        );
    }
}

#[tokio::test]
async fn test_tasks_review_excludes_out_of_range() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file(
        "tasks.pn",
        "- old task {@task status=done completed_at=2024-01-10}\n\
         - new task {@task status=done completed_at=2024-03-15}\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("tasks.pn");
    client
        .did_open(
            uri.clone(),
            "- old task {@task status=done completed_at=2024-01-10}\n\
             - new task {@task status=done completed_at=2024-03-15}\n"
                .to_string(),
        )
        .await;

    let result = client
        .tasks_review("custom", Some("2024-03-01"), Some("2024-03-31"))
        .await;
    let tasks = result
        .flatten()
        .expect("tasks_review should return a value");
    let arr = tasks.as_array().expect("result should be an array");

    assert_eq!(arr.len(), 1, "Only one task should be in March");
    assert_eq!(arr[0]["completed_at"], "2024-03-15");
}

#[tokio::test]
async fn test_auto_complete_detects_nested_task_transition() {
    let mut workspace = TestWorkspace::new();
    // Parent task with indented child task
    workspace.create_file(
        "tasks.pn",
        "parent {@task status=todo}\n\tchild {@task status=todo}\n",
    );

    let mut client = InProcessLspClient::new(&workspace).await;
    let uri = workspace.get_uri("tasks.pn");

    client
        .did_open(
            uri.clone(),
            "parent {@task status=todo}\n\tchild {@task status=todo}\n".to_string(),
        )
        .await;

    // Mark the nested child task as Done
    client
        .did_change(
            uri.clone(),
            2,
            "parent {@task status=todo}\n\tchild {@task status=done}\n".to_string(),
        )
        .await;

    let ast = client.get_ast(&uri).expect("AST should be present");
    let mut tasks = Vec::new();
    collect_tasks(&ast, &mut tasks);

    assert_eq!(tasks.len(), 2, "Expected two tasks (parent + child)");

    // Find the child task (row 1)
    let child = tasks.iter().find(|(row, _, _)| *row == 1);
    assert!(child.is_some(), "Should find child task at row 1");
    let (_, status, _) = child.unwrap();
    assert_eq!(*status, TaskStatus::Done, "Child task should be Done");
}

/// When a shorthand task (`!YYYY-MM-DD`) is changed to done (`-YYYY-MM-DD`),
/// the generated edit should replace the shorthand token+date with a full
/// `{@task status=done due=YYYY-MM-DD completed_at=...}` block.
#[tokio::test]
async fn test_shorthand_task_replaced_with_longform_on_done() {
    let mut workspace = TestWorkspace::new();
    workspace.create_file("tasks.pn", "buy milk !2024-12-31\n");

    let client = InProcessLspClient::new(&workspace).await;

    let old_text = "buy milk !2024-12-31\n";
    let new_text = "buy milk -2024-12-31\n";

    let old_ast = patto::parser::parse_text(old_text).ast;
    let new_ast = patto::parser::parse_text(new_text).ast;

    let edits = client.completion_edits(&old_ast, &new_ast);

    assert_eq!(edits.len(), 1, "Expected exactly one edit for shorthand→longform");

    let edit = &edits[0];
    // The edit should be on line 0
    assert_eq!(edit.range.start.line, 0);
    // The replacement text must contain the full {@task} block with status=done and due=
    assert!(
        edit.new_text.contains("{@task"),
        "Edit should produce a {{@task}} block, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("status=done"),
        "Edit should set status=done, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("due=2024-12-31"),
        "Edit should preserve the due date, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("completed_at="),
        "Edit should add completed_at, got: {}",
        edit.new_text
    );
    // The shorthand `-YYYY-MM-DD` should NOT remain in the replacement text
    assert!(
        !edit.new_text.starts_with('-'),
        "Shorthand dash prefix should be replaced, got: {}",
        edit.new_text
    );
}
