//! Sync between a phone clone and a "desktop" clone through a bare origin on
//! disk, so no network is involved.

use std::collections::HashSet;
use std::path::Path;

use git2::{IndexAddOption, Oid, Repository, RepositoryInitOptions, Signature};
use tempfile::TempDir;

use crate::api::conflict::*;
use crate::api::error::{GitErrorKind, PattoError};
use crate::api::git::*;
use crate::api::merge::{assemble, MergeRegion, SuggestionKind};

struct Setup {
    origin: TempDir,
    desktop_dir: TempDir,
    phone_dir: TempDir,
}

fn creds() -> GitCreds {
    GitCreds {
        username: String::new(),
        token: String::new(),
    }
}

fn write(root: &Path, rel: &str, content: &str) {
    std::fs::write(root.join(rel), content).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap()
}

fn commit_all(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = Signature::now("Desktop", "desktop@example.com").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<_> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap()
}

fn push_main(repo: &Repository) {
    repo.find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();
}

fn origin_ref(setup: &Setup, name: &str) -> Option<Oid> {
    let origin = Repository::open(setup.origin.path()).unwrap();
    let target = origin.find_reference(name).ok()?.target();
    target
}

fn origin_file(setup: &Setup, rel: &str) -> String {
    let origin = Repository::open(setup.origin.path()).unwrap();
    let tree = origin
        .find_reference("refs/heads/main")
        .unwrap()
        .peel_to_tree()
        .unwrap();
    let entry = tree.get_path(Path::new(rel)).unwrap();
    let blob = origin.find_blob(entry.id()).unwrap();
    String::from_utf8(blob.content().to_vec()).unwrap()
}

impl Setup {
    /// Origin with two notes, cloned to a desktop and a phone.
    fn new() -> Self {
        let origin = tempfile::tempdir().unwrap();
        let mut opts = RepositoryInitOptions::new();
        opts.bare(true).initial_head("main");
        Repository::init_opts(origin.path(), &opts).unwrap();

        let desktop_dir = tempfile::tempdir().unwrap();
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");
        let desktop = Repository::init_opts(desktop_dir.path(), &opts).unwrap();
        desktop
            .remote("origin", origin.path().to_str().unwrap())
            .unwrap();
        write(desktop_dir.path(), "shopping.pn", "bread\nmilk\neggs\n");
        write(desktop_dir.path(), "other.pn", "x\n");
        commit_all(&desktop, "seed");
        push_main(&desktop);

        let phone_dir = tempfile::tempdir().unwrap();
        let phone_root = phone_dir.path().join("notes");
        git_clone(
            origin.path().to_str().unwrap().to_string(),
            phone_root.to_str().unwrap().to_string(),
            Some("main".to_string()),
            creds(),
            |_| {},
        )
        .unwrap();

        Setup {
            origin,
            desktop_dir,
            phone_dir,
        }
    }

    fn desktop(&self) -> Repository {
        Repository::open(self.desktop_dir.path()).unwrap()
    }

    fn phone_root(&self) -> std::path::PathBuf {
        self.phone_dir.path().join("notes")
    }

    fn phone(&self) -> String {
        self.phone_root().to_str().unwrap().to_string()
    }

    fn desktop_edit(&self, rel: &str, content: &str) {
        write(self.desktop_dir.path(), rel, content);
        let repo = self.desktop();
        commit_all(&repo, &format!("desktop: edit {rel}"));
        push_main(&repo);
    }

    fn phone_edit(&self, rel: &str, content: &str) {
        write(&self.phone_root(), rel, content);
    }

    fn sync(&self) -> Result<SyncReport, PattoError> {
        git_sync(
            self.phone(),
            "Phone".to_string(),
            "phone@example.com".to_string(),
            creds(),
            |_| {},
        )
    }

    fn resolve(&self, resolutions: Vec<Resolution>) -> Result<SyncReport, PattoError> {
        git_resolve(
            self.phone(),
            "Phone".to_string(),
            "phone@example.com".to_string(),
            creds(),
            resolutions,
            |_| {},
        )
    }

    /// Desktop edits the milk line one way, the phone another, and the phone
    /// syncs. Also changes `other.pn` on the desktop, which merges cleanly.
    fn pause_on_conflict(&self) -> SyncReport {
        self.desktop_edit("other.pn", "y\n");
        self.desktop_edit("shopping.pn", "bread\noat milk\neggs\n");
        self.phone_edit("shopping.pn", "bread\nmilk {@task status=done}\neggs\n");
        self.sync().unwrap()
    }
}

#[test]
fn edits_to_different_notes_merge_and_push() {
    let setup = Setup::new();
    setup.desktop_edit("other.pn", "y\n");
    setup.phone_edit("shopping.pn", "bread\nmilk\neggs\nbutter\n");

    let report = setup.sync().unwrap();
    assert_eq!(report.merge, MergeOutcome::Merged);
    assert!(report.pushed);
    assert!(!report.conflict_cleared);
    assert_eq!(origin_file(&setup, "other.pn"), "y\n");
    assert_eq!(
        origin_file(&setup, "shopping.pn"),
        "bread\nmilk\neggs\nbutter\n"
    );
    assert_eq!(read(&setup.phone_root(), "other.pn"), "y\n");
}

#[test]
fn attachments_are_committed_and_pushed_with_the_notes() {
    let setup = Setup::new();
    let attachments = setup.phone_root().join("attachments").join("sub");
    std::fs::create_dir_all(&attachments).unwrap();
    std::fs::write(attachments.join("photo.png"), "fake png\n").unwrap();
    write(&setup.phone_root(), "stray.txt", "not synced\n");
    setup.phone_edit("other.pn", "[@img ./attachments/sub/photo.png]\n");

    let status = git_status(setup.phone()).unwrap();
    assert_eq!(status.dirty, vec!["attachments/sub/photo.png", "other.pn"]);
    assert_eq!(
        locally_modified_notes(&setup.phone()).unwrap(),
        HashSet::from(["other.pn".to_string()])
    );

    let report = setup.sync().unwrap();
    assert!(report.pushed);
    assert_eq!(
        origin_file(&setup, "attachments/sub/photo.png"),
        "fake png\n"
    );
    let origin = Repository::open(setup.origin.path()).unwrap();
    let tree = origin
        .find_reference("refs/heads/main")
        .unwrap()
        .peel_to_tree()
        .unwrap();
    assert!(tree.get_path(Path::new("stray.txt")).is_err());
}

#[test]
fn a_conflict_pauses_the_sync_and_pushes_a_side_branch() {
    let setup = Setup::new();
    let desktop_tip = {
        setup.desktop_edit("other.pn", "y\n");
        setup.desktop_edit("shopping.pn", "bread\noat milk\neggs\n");
        setup.desktop().head().unwrap().target().unwrap()
    };
    setup.phone_edit("shopping.pn", "bread\nmilk {@task status=done}\neggs\n");
    let report = setup.sync().unwrap();

    let MergeOutcome::Conflicted { side_branch, paths } = &report.merge else {
        panic!("expected a conflict, got {:?}", report.merge);
    };
    assert!(side_branch.starts_with("mobile/"));
    assert_eq!(paths, &vec!["shopping.pn".to_string()]);
    assert!(!report.pushed);

    // Nothing merged: the phone keeps its own text and the remote is untouched.
    assert_eq!(
        read(&setup.phone_root(), "shopping.pn"),
        "bread\nmilk {@task status=done}\neggs\n"
    );
    assert_eq!(read(&setup.phone_root(), "other.pn"), "x\n");
    assert_eq!(origin_ref(&setup, "refs/heads/main"), Some(desktop_tip));

    // The phone's commit is safe on the remote.
    let phone_head = Repository::open(setup.phone_root())
        .unwrap()
        .head()
        .unwrap()
        .target();
    assert_eq!(
        origin_ref(&setup, &format!("refs/heads/{side_branch}")),
        phone_head
    );

    assert!(git_status(setup.phone()).unwrap().conflict_pending);
    let pending = pending_conflict(setup.phone()).unwrap().unwrap();
    assert_eq!(&pending.side_branch, side_branch);
    assert_eq!(pending.remote.id, desktop_tip.to_string());
    assert_eq!(pending.files.len(), 1);
    assert_eq!(pending.files[0].kind, ConflictKind::BothModified);
    assert_eq!(pending.files[0].conflicts, 1);
    assert_eq!(pending.held_back, vec!["other.pn".to_string()]);
}

#[test]
fn syncing_again_while_paused_stays_paused_and_updates_the_side_branch() {
    let setup = Setup::new();
    setup.pause_on_conflict();

    setup.phone_edit("other.pn", "x\nfrom the phone\n");
    let report = setup.sync().unwrap();
    let MergeOutcome::Conflicted { side_branch, .. } = &report.merge else {
        panic!("expected a conflict, got {:?}", report.merge);
    };
    assert!(report.committed);
    let phone_head = Repository::open(setup.phone_root())
        .unwrap()
        .head()
        .unwrap()
        .target();
    assert_eq!(
        origin_ref(&setup, &format!("refs/heads/{side_branch}")),
        phone_head
    );
}

#[test]
fn resolving_on_the_phone_merges_pushes_and_removes_the_side_branch() {
    let setup = Setup::new();
    let report = setup.pause_on_conflict();
    let MergeOutcome::Conflicted { side_branch, .. } = report.merge else {
        panic!("expected a conflict");
    };

    let detail = conflict_detail(setup.phone(), "shopping.pn".to_string()).unwrap();
    assert_eq!(
        detail.ours.as_deref(),
        Some("bread\nmilk {@task status=done}\neggs\n")
    );
    assert_eq!(detail.theirs.as_deref(), Some("bread\noat milk\neggs\n"));
    let suggestion = detail
        .merged
        .regions
        .iter()
        .find_map(|r| match r {
            MergeRegion::Conflict { suggestion, .. } => suggestion.clone(),
            _ => None,
        })
        .unwrap();
    assert_eq!(suggestion.kind, SuggestionKind::Combined);

    let content = assemble(&detail.merged, &[suggestion.lines]);
    assert_eq!(content, "bread\noat milk {@task status=done}\neggs\n");

    let report = setup
        .resolve(vec![Resolution {
            path: detail.path,
            ours_id: detail.ours_id,
            theirs_id: detail.theirs_id,
            content: Some(content.clone()),
        }])
        .unwrap();
    assert!(report.pushed);
    assert!(report.conflict_cleared);

    assert_eq!(origin_file(&setup, "shopping.pn"), content);
    assert_eq!(origin_file(&setup, "other.pn"), "y\n");
    assert_eq!(read(&setup.phone_root(), "shopping.pn"), content);
    assert_eq!(read(&setup.phone_root(), "other.pn"), "y\n");
    assert_eq!(
        origin_ref(&setup, &format!("refs/heads/{side_branch}")),
        None
    );
    assert!(!git_status(setup.phone()).unwrap().conflict_pending);
    assert_eq!(pending_conflict(setup.phone()).unwrap(), None);
}

#[test]
fn a_merge_done_on_the_desktop_is_picked_up_by_the_next_sync() {
    let setup = Setup::new();
    let report = setup.pause_on_conflict();
    let MergeOutcome::Conflicted { side_branch, .. } = report.merge else {
        panic!("expected a conflict");
    };

    // The desktop merges the side branch by hand.
    let desktop = setup.desktop();
    desktop
        .find_remote("origin")
        .unwrap()
        .fetch(
            &[&format!(
                "refs/heads/{side_branch}:refs/remotes/origin/{side_branch}"
            )],
            None,
            None,
        )
        .unwrap();
    let side_tip = desktop
        .find_reference(&format!("refs/remotes/origin/{side_branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    write(
        setup.desktop_dir.path(),
        "shopping.pn",
        "bread\noat milk (desktop)\neggs\n",
    );
    let mut index = desktop.index().unwrap();
    index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree = desktop.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = Signature::now("Desktop", "desktop@example.com").unwrap();
    let head = desktop.head().unwrap().peel_to_commit().unwrap();
    desktop
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "merge phone",
            &tree,
            &[&head, &side_tip],
        )
        .unwrap();
    push_main(&desktop);

    let report = setup.sync().unwrap();
    assert_eq!(report.merge, MergeOutcome::FastForward);
    assert!(report.conflict_cleared);
    assert_eq!(
        read(&setup.phone_root(), "shopping.pn"),
        "bread\noat milk (desktop)\neggs\n"
    );
    assert_eq!(read(&setup.phone_root(), "other.pn"), "y\n");
    assert_eq!(
        origin_ref(&setup, &format!("refs/heads/{side_branch}")),
        None
    );
    assert!(!git_status(setup.phone()).unwrap().conflict_pending);
}

#[test]
fn a_resolution_is_refused_when_the_remote_moved() {
    let setup = Setup::new();
    setup.pause_on_conflict();
    let detail = conflict_detail(setup.phone(), "shopping.pn".to_string()).unwrap();

    setup.desktop_edit("other.pn", "z\n");

    let err = setup
        .resolve(vec![Resolution {
            path: detail.path,
            ours_id: detail.ours_id,
            theirs_id: detail.theirs_id,
            content: Some("anything\n".to_string()),
        }])
        .unwrap_err();
    assert!(
        matches!(
            err,
            PattoError::Git {
                kind: GitErrorKind::Stale,
                ..
            }
        ),
        "{err:?}"
    );
    // Nothing was merged.
    assert!(git_status(setup.phone()).unwrap().conflict_pending);
    assert_eq!(read(&setup.phone_root(), "other.pn"), "x\n");
}

#[test]
fn a_resolution_is_refused_when_the_note_changed_since_review() {
    let setup = Setup::new();
    setup.pause_on_conflict();
    let detail = conflict_detail(setup.phone(), "shopping.pn".to_string()).unwrap();

    setup.phone_edit(
        "shopping.pn",
        "bread\nmilk {@task status=done}\neggs\ntea\n",
    );

    let err = setup
        .resolve(vec![Resolution {
            path: detail.path,
            ours_id: detail.ours_id,
            theirs_id: detail.theirs_id,
            content: Some("anything\n".to_string()),
        }])
        .unwrap_err();
    assert!(
        matches!(
            err,
            PattoError::Git {
                kind: GitErrorKind::Stale,
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_note_deleted_on_the_remote_can_be_kept() {
    let setup = Setup::new();
    {
        let desktop = setup.desktop();
        std::fs::remove_file(setup.desktop_dir.path().join("shopping.pn")).unwrap();
        commit_all(&desktop, "delete shopping");
        push_main(&desktop);
    }
    setup.phone_edit("shopping.pn", "bread\nmilk\neggs\ntea\n");
    let report = setup.sync().unwrap();
    assert!(matches!(report.merge, MergeOutcome::Conflicted { .. }));

    let detail = conflict_detail(setup.phone(), "shopping.pn".to_string()).unwrap();
    assert_eq!(detail.kind, ConflictKind::DeletedByThem);
    assert_eq!(detail.theirs, None);

    setup
        .resolve(vec![Resolution {
            path: detail.path,
            ours_id: detail.ours_id,
            theirs_id: detail.theirs_id,
            content: detail.ours.clone(),
        }])
        .unwrap();
    assert_eq!(
        origin_file(&setup, "shopping.pn"),
        "bread\nmilk\neggs\ntea\n"
    );
}
