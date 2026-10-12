use std::path::{Path, PathBuf};
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

/// Whether `message` announces `name` appearing in the repository.
///
/// Writing a new file produces a create and a write, and which of them the
/// platform reports first decides whether the repository sends `FileAdded` or
/// the debounced `FileChanged`. Either means the note was picked up.
fn announces(message: &RepositoryMessage, name: &str) -> bool {
    match message {
        RepositoryMessage::FileAdded(path, _) => is_named(path, name),
        RepositoryMessage::FileChanged(path, _, _) => is_named(path, name),
        _ => false,
    }
}

/// A temporary notes directory, by its canonical path.
///
/// The watcher decides whether an event belongs to the repository with
/// `path.starts_with(root_dir)`, and the filesystem reports canonical paths.
/// On macOS a temporary directory is reached through a symlink — `/var/...`
/// for a real `/private/var/...` — so an uncanonicalised root matches nothing.
/// Both binaries canonicalise before constructing a `Repository`; so do these
/// tests.
fn notes_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap();

    // On Windows `canonicalize` returns an extended-length path (`\\?\C:\...`),
    // which the watcher's events are not reported with.
    let path = match path.to_str().and_then(|path| path.strip_prefix(r"\\?\")) {
        Some(stripped) => PathBuf::from(stripped),
        None => path,
    };
    (dir, path)
}

#[tokio::test]
async fn watcher_reports_create_modify_and_remove() {
    let (_dir, dir) = notes_dir();
    std::fs::write(dir.join("a.pn"), "first note\n").unwrap();

    let repository = Repository::new(dir.clone());
    let mut rx = repository.subscribe();
    repository.start_watcher().await.unwrap();

    let created = dir.join("b.pn");
    std::fs::write(&created, "second note\n").unwrap();
    wait_for(&mut rx, |m| announces(m, "b.pn")).await;

    std::fs::write(dir.join("a.pn"), "first note, edited\n").unwrap();
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
    let (_dir, dir) = notes_dir();
    let repository = Repository::new(dir.clone());
    let mut rx = repository.subscribe();
    repository.start_watcher().await.unwrap();

    std::fs::write(dir.join("notes.txt"), "ignored\n").unwrap();
    std::fs::write(dir.join("c.pn"), "watched\n").unwrap();

    // The `.pn` file arriving without a preceding message for `notes.txt` is
    // what proves the extension filter holds.
    let message = wait_for(&mut rx, |m| {
        announces(m, "c.pn") || announces(m, "notes.txt")
    })
    .await;
    assert!(
        announces(&message, "c.pn"),
        "the watcher should ignore non-patto files, got {message:?}"
    );
}

#[tokio::test]
async fn watcher_reloads_the_workspace_config() {
    let (_dir, dir) = notes_dir();
    let repository = Repository::new(dir.clone());
    let mut rx = repository.subscribe();
    repository.start_watcher().await.unwrap();

    std::fs::write(dir.join(".patto.toml"), "pinned_files = [\"a.pn\"]\n").unwrap();

    let message = wait_for(&mut rx, |m| {
        matches!(m, RepositoryMessage::WorkspaceConfigChanged(_))
    })
    .await;
    let RepositoryMessage::WorkspaceConfigChanged(config) = message else {
        unreachable!()
    };
    assert_eq!(config.pinned_files, vec!["a.pn".to_string()]);
}
