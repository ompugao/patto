//! Sync against a local bare repository.
//!
//! `file://` transport needs no credentials, so these exercise the commit,
//! merge and push logic without a network or a token.

use std::path::Path;

use rust_lib_patto_flutter::api::git::*;

struct Remote {
    dir: tempfile::TempDir,
}

impl Remote {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init_bare(dir.path()).unwrap();
        Remote { dir }
    }

    fn url(&self) -> String {
        format!("file://{}", self.dir.path().display())
    }
}

fn creds() -> GitCreds {
    GitCreds {
        username: String::new(),
        token: String::new(),
    }
}

fn clone_to(remote: &Remote, into: &Path) {
    git_clone(
        remote.url(),
        into.to_string_lossy().to_string(),
        None,
        creds(),
        |_| {},
    )
    .unwrap();
}

fn sync(root: &Path) -> SyncReport {
    git_sync(
        root.to_string_lossy().to_string(),
        "Tester".to_string(),
        "tester@example.com".to_string(),
        creds(),
        |_| {},
    )
    .unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    std::fs::write(root.join(rel), content).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// A bare remote has no branch until something is pushed, so seed it from a
/// first working copy.
fn seeded_remote() -> (Remote, tempfile::TempDir) {
    let remote = Remote::new();
    let seed = tempfile::tempdir().unwrap();

    let repo = git2::Repository::init(seed.path()).unwrap();
    write(seed.path(), "seed.pn", "seed note\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("seed.pn")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Seed", "seed@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "seed", &tree, &[])
        .unwrap();

    let mut origin = repo.remote("origin", &remote.url()).unwrap();
    let head = repo.head().unwrap();
    let branch = head.shorthand().unwrap().to_string();
    origin
        .push(&[&format!("refs/heads/{branch}:refs/heads/{branch}")], None)
        .unwrap();

    (remote, seed)
}

#[test]
fn cloning_brings_the_notes_down() {
    let (remote, _seed) = seeded_remote();
    let local = tempfile::tempdir().unwrap();

    clone_to(&remote, local.path());
    assert_eq!(read(local.path(), "seed.pn"), "seed note\n");
}

#[test]
fn a_clean_tree_commits_nothing() {
    let (remote, _seed) = seeded_remote();
    let local = tempfile::tempdir().unwrap();
    clone_to(&remote, local.path());

    let report = sync(local.path());
    assert!(!report.committed);
    assert_eq!(report.merge, MergeOutcome::UpToDate);
}

#[test]
fn local_edits_are_committed_and_pushed() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    write(a.path(), "new.pn", "from a\n");
    let report = sync(a.path());
    assert!(report.committed);
    assert!(report.pushed);

    let report = sync(b.path());
    assert_eq!(report.merge, MergeOutcome::FastForward);
    assert_eq!(read(b.path(), "new.pn"), "from a\n");
}

#[test]
fn a_deleted_note_is_staged_too() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    std::fs::remove_file(a.path().join("seed.pn")).unwrap();
    assert!(sync(a.path()).committed);

    sync(b.path());
    assert!(!b.path().join("seed.pn").exists());
}

#[test]
fn edits_to_different_notes_merge() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    write(a.path(), "from-a.pn", "a\n");
    sync(a.path());

    write(b.path(), "from-b.pn", "b\n");
    let report = sync(b.path());

    assert!(matches!(report.merge, MergeOutcome::Merged { .. }));
    assert!(report.pushed);
    assert_eq!(read(b.path(), "from-a.pn"), "a\n");
    assert_eq!(read(b.path(), "from-b.pn"), "b\n");
}

#[test]
fn a_conflicting_edit_keeps_the_local_copy() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    write(a.path(), "seed.pn", "edited on a\n");
    sync(a.path());

    write(b.path(), "seed.pn", "edited on b\n");
    let report = sync(b.path());

    assert!(report.pushed);
    assert_eq!(read(b.path(), "seed.pn"), "edited on b\n");
    // The file is merged cleanly in our favour, so no conflict entry remains.
    let repo = git2::Repository::open(b.path()).unwrap();
    assert!(!repo.index().unwrap().has_conflicts());
}

#[test]
fn a_note_edited_here_and_deleted_there_survives() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    std::fs::remove_file(a.path().join("seed.pn")).unwrap();
    sync(a.path());

    write(b.path(), "seed.pn", "still wanted\n");
    let report = sync(b.path());

    assert!(report.pushed);
    assert_eq!(read(b.path(), "seed.pn"), "still wanted\n");
}

#[test]
fn status_reports_the_branch_and_dirty_notes() {
    let (remote, _seed) = seeded_remote();
    let local = tempfile::tempdir().unwrap();
    clone_to(&remote, local.path());

    let clean = git_status(local.path().to_string_lossy().to_string()).unwrap();
    assert!(clean.dirty.is_empty());
    assert!(clean.has_remote);
    assert!(!clean.branch.is_empty());

    write(local.path(), "draft.pn", "unsaved\n");
    let dirty = git_status(local.path().to_string_lossy().to_string()).unwrap();
    assert_eq!(dirty.dirty, vec!["draft.pn".to_string()]);
}

#[test]
fn status_counts_how_far_behind_we_are() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    write(a.path(), "ahead.pn", "x\n");
    sync(a.path());

    let repo = git2::Repository::open(b.path()).unwrap();
    let branch = repo.head().unwrap().shorthand().unwrap().to_string();
    repo.find_remote("origin")
        .unwrap()
        .fetch(&[&format!("refs/heads/{branch}")], None, None)
        .unwrap();

    let status = git_status(b.path().to_string_lossy().to_string()).unwrap();
    assert_eq!(status.behind, 1);
    assert_eq!(status.ahead, 0);
}

#[test]
fn sync_reports_which_notes_changed() {
    let (remote, _seed) = seeded_remote();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    clone_to(&remote, a.path());
    clone_to(&remote, b.path());

    write(a.path(), "changed.pn", "x\n");
    sync(a.path());

    let report = sync(b.path());
    assert!(report.changed_paths.contains(&"changed.pn".to_string()));
}

#[test]
fn syncing_a_directory_that_is_not_a_repository_fails() {
    let dir = tempfile::tempdir().unwrap();
    let err = git_sync(
        dir.path().to_string_lossy().to_string(),
        "Tester".to_string(),
        "tester@example.com".to_string(),
        creds(),
        |_| {},
    )
    .unwrap_err();
    assert!(matches!(
        err,
        rust_lib_patto_flutter::api::error::PattoError::Git { .. }
    ));
}
