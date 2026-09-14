use std::path::Path;
use std::time::Duration;

use patto::repository::{Repository, RepositoryMessage};
use tokio::sync::broadcast::Receiver;
use tokio::time::timeout;

/// Wait for the first message matching `predicate`, ignoring the rest.
async fn wait_for<F>(rx: &mut Receiver<RepositoryMessage>, predicate: F) -> RepositoryMessage
where
    F: Fn(&RepositoryMessage) -> bool,
{
    let deadline = Duration::from_secs(10);
    timeout(deadline, async {
        loop {
            match rx.recv().await {
                Ok(message) if predicate(&message) => return message,
                Ok(_) => continue,
                Err(err) => panic!("repository channel closed: {err}"),
            }
        }
    })
    .await
    .expect("timed out waiting for the expected repository message")
}

fn is_named(path: &Path, name: &str) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some(name)
}

#[tokio::test]
async fn watcher_reports_create_modify_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pn"), "first note\n").unwrap();

    let repository = Repository::new(dir.path().to_path_buf());
    let mut rx = repository.subscribe();
    repository.start_watcher().await.unwrap();

    let created = dir.path().join("b.pn");
    std::fs::write(&created, "second note\n").unwrap();
    let message = wait_for(
        &mut rx,
        |m| matches!(m, RepositoryMessage::FileAdded(path, _) if is_named(path, "b.pn")),
    )
    .await;
    assert!(matches!(message, RepositoryMessage::FileAdded(..)));

    std::fs::write(dir.path().join("a.pn"), "first note, edited\n").unwrap();
    let message = wait_for(
        &mut rx,
        |m| matches!(m, RepositoryMessage::FileChanged(path, _, _) if is_named(path, "a.pn")),
    )
    .await;
    let RepositoryMessage::FileChanged(_, _, content) = message else {
        unreachable!()
    };
    assert_eq!(content, "first note, edited\n");

    std::fs::remove_file(&created).unwrap();
    wait_for(
        &mut rx,
        |m| matches!(m, RepositoryMessage::FileRemoved(path) if is_named(path, "b.pn")),
    )
    .await;
}

#[tokio::test]
async fn watcher_ignores_non_patto_files() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Repository::new(dir.path().to_path_buf());
    let mut rx = repository.subscribe();
    repository.start_watcher().await.unwrap();

    std::fs::write(dir.path().join("notes.txt"), "ignored\n").unwrap();
    std::fs::write(dir.path().join("c.pn"), "watched\n").unwrap();

    // The `.pn` file arriving without a preceding message for `notes.txt` is
    // what proves the extension filter holds.
    let message = wait_for(&mut rx, |m| matches!(m, RepositoryMessage::FileAdded(..))).await;
    let RepositoryMessage::FileAdded(path, _) = message else {
        unreachable!()
    };
    assert!(is_named(&path, "c.pn"));
}

#[tokio::test]
async fn watcher_reloads_the_workspace_config() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Repository::new(dir.path().to_path_buf());
    let mut rx = repository.subscribe();
    repository.start_watcher().await.unwrap();

    std::fs::write(
        dir.path().join(".patto.toml"),
        "pinned_files = [\"a.pn\"]\n",
    )
    .unwrap();

    let message = wait_for(&mut rx, |m| {
        matches!(m, RepositoryMessage::WorkspaceConfigChanged(_))
    })
    .await;
    let RepositoryMessage::WorkspaceConfigChanged(config) = message else {
        unreachable!()
    };
    assert_eq!(config.pinned_files, vec!["a.pn".to_string()]);
}
