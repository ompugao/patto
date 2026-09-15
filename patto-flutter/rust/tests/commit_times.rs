//! Note timestamps come from git, because a clone does not preserve file times.

use std::path::Path;

use rust_lib_patto_flutter::api::index::apply_commit_times;
use rust_lib_patto_flutter::api::store::list_notes;
use rust_lib_patto_flutter::api::types::NoteMeta;

struct Repo {
    dir: tempfile::TempDir,
    repo: git2::Repository,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        Repo { dir, repo }
    }

    fn root(&self) -> String {
        self.dir.path().to_string_lossy().to_string()
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// Commit everything with an explicit timestamp, in seconds since the epoch.
    fn commit(&self, message: &str, at: i64) {
        let mut index = self.repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = self.repo.find_tree(index.write_tree().unwrap()).unwrap();

        let when = git2::Time::new(at, 0);
        let sig = git2::Signature::new("Tester", "tester@example.com", &when).unwrap();
        let parent = self.repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        self.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap();
    }

    fn notes(&self) -> Vec<NoteMeta> {
        let mut notes = list_notes(self.root()).unwrap();
        apply_commit_times(&self.root(), &mut notes).unwrap();
        notes
    }

    fn time_of(&self, rel_path: &str) -> i64 {
        self.notes()
            .into_iter()
            .find(|n| n.rel_path == rel_path)
            .unwrap_or_else(|| panic!("{rel_path} is not listed"))
            .modified_ms
    }
}

/// Every file in a clone carries the time of the clone, so a file time that
/// differs from the commit time is what this is all about.
fn far_past() -> i64 {
    1_600_000_000
}

#[test]
fn a_committed_note_reports_when_it_was_committed() {
    let repo = Repo::new();
    repo.write("a.pn", "hello\n");
    repo.commit("add a", far_past());

    assert_eq!(repo.time_of("a.pn"), far_past() * 1000);
}

#[test]
fn the_newest_commit_touching_a_note_wins() {
    let repo = Repo::new();
    repo.write("a.pn", "first\n");
    repo.commit("add a", far_past());

    repo.write("a.pn", "second\n");
    repo.commit("edit a", far_past() + 5_000);

    assert_eq!(repo.time_of("a.pn"), (far_past() + 5_000) * 1000);
}

#[test]
fn a_note_untouched_by_the_latest_commit_keeps_its_own_time() {
    let repo = Repo::new();
    repo.write("old.pn", "old\n");
    repo.commit("add old", far_past());

    repo.write("new.pn", "new\n");
    repo.commit("add new", far_past() + 9_000);

    assert_eq!(repo.time_of("old.pn"), far_past() * 1000);
    assert_eq!(repo.time_of("new.pn"), (far_past() + 9_000) * 1000);
}

#[test]
fn a_locally_edited_note_keeps_its_file_time() {
    let repo = Repo::new();
    repo.write("a.pn", "committed\n");
    repo.commit("add a", far_past());

    repo.write("a.pn", "edited here, not committed\n");

    // The edit just happened, so the file time is far newer than the commit.
    assert!(repo.time_of("a.pn") > far_past() * 1000);
}

#[test]
fn an_untracked_note_keeps_its_file_time() {
    let repo = Repo::new();
    repo.write("tracked.pn", "committed\n");
    repo.commit("add tracked", far_past());

    repo.write("draft.pn", "never committed\n");

    assert_eq!(repo.time_of("tracked.pn"), far_past() * 1000);
    assert!(repo.time_of("draft.pn") > far_past() * 1000);
}

#[test]
fn notes_in_subdirectories_are_matched_too() {
    let repo = Repo::new();
    repo.write("dir/deep.pn", "nested\n");
    repo.commit("add nested", far_past());

    assert_eq!(repo.time_of("dir/deep.pn"), far_past() * 1000);
}

#[test]
fn a_directory_that_is_not_a_repository_is_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pn"), "hello\n").unwrap();
    let root = dir.path().to_string_lossy().to_string();

    let mut notes = list_notes(root.clone()).unwrap();
    let before = notes[0].modified_ms;
    apply_commit_times(&root, &mut notes).unwrap();

    assert_eq!(notes[0].modified_ms, before);
}

#[test]
fn a_repository_with_no_commits_yet_is_left_alone() {
    let repo = Repo::new();
    repo.write("a.pn", "hello\n");

    let mut notes = list_notes(repo.root()).unwrap();
    let before = notes[0].modified_ms;
    apply_commit_times(&repo.root(), &mut notes).unwrap();

    assert_eq!(notes[0].modified_ms, before);
}

/// The case that prompted this: a clone stamps every file with the clone time,
/// so ordering by file time is meaningless.
#[test]
fn a_clone_still_orders_notes_by_when_they_were_written() {
    let origin = Repo::new();
    origin.write("oldest.pn", "1\n");
    origin.commit("add oldest", far_past());
    origin.write("middle.pn", "2\n");
    origin.commit("add middle", far_past() + 1_000);
    origin.write("newest.pn", "3\n");
    origin.commit("add newest", far_past() + 2_000);

    let target = tempfile::tempdir().unwrap();
    git2::Repository::clone(
        &format!("file://{}", origin.dir.path().display()),
        target.path(),
    )
    .unwrap();

    let root = target.path().to_string_lossy().to_string();
    let mut notes = list_notes(root.clone()).unwrap();

    // Every file was written by the clone, so the filesystem cannot tell them
    // apart.
    let file_times: Vec<i64> = notes.iter().map(|n| n.modified_ms).collect();
    let spread = file_times.iter().max().unwrap() - file_times.iter().min().unwrap();
    assert!(spread < 5_000, "clone times differ by {spread}ms");

    apply_commit_times(&root, &mut notes).unwrap();
    notes.sort_by_key(|n| std::cmp::Reverse(n.modified_ms));

    let order: Vec<&str> = notes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(order, vec!["newest", "middle", "oldest"]);
}

#[test]
fn a_renamed_note_reports_the_rename_commit() {
    let repo = Repo::new();
    repo.write("before.pn", "content\n");
    repo.commit("add", far_past());

    std::fs::rename(
        repo.dir.path().join("before.pn"),
        repo.dir.path().join("after.pn"),
    )
    .unwrap();
    let mut index = repo.repo.index().unwrap();
    index.remove_path(Path::new("before.pn")).unwrap();
    index.write().unwrap();
    repo.commit("rename", far_past() + 7_000);

    assert_eq!(repo.time_of("after.pn"), (far_past() + 7_000) * 1000);
}
