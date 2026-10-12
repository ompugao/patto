//! The link graph a repository builds from its notes, as the preview, the LSP
//! and the TUI read it.
#![cfg(feature = "repository")]

use std::collections::HashMap;
use std::path::PathBuf;

use patto::repository::{load_workspace_config, Repository, RepositoryMessage};
use url::Url;

/// A temporary notes directory by its canonical path: the graph keys notes by
/// the URI of the path it was given, and back-link lookups canonicalise first.
fn notes_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap();
    (dir, path)
}

fn repository_with(notes: &[(&str, &str)]) -> (tempfile::TempDir, Repository) {
    let (dir, root) = notes_dir();
    for (name, content) in notes {
        std::fs::write(root.join(name), content).unwrap();
    }
    let repository = Repository::new(root.clone());
    for (name, content) in notes {
        repository.add_file_to_graph(&root.join(name), content);
    }
    (dir, repository)
}

#[test]
fn back_links_list_every_note_linking_to_a_file_sorted_by_name() {
    let (_dir, repository) = repository_with(&[
        ("c.pn", "[b]\n"),
        ("a.pn", "[b]\n[b#section]\n"),
        ("b.pn", "target\n"),
    ]);

    let back_links = repository.calculate_back_links(&repository.root_dir.join("b.pn"));

    let sources: Vec<&str> = back_links.iter().map(|b| b.source_file.as_str()).collect();
    assert_eq!(sources, ["a", "c"]);
    assert_eq!(back_links[0].locations.len(), 2);
    assert_eq!(back_links[0].locations[1].line, 1);
    assert_eq!(
        back_links[0].locations[1].target_anchor.as_deref(),
        Some("section")
    );
    assert_eq!(
        repository.count_back_links(&repository.root_dir.join("b.pn")),
        3
    );
}

#[test]
fn back_links_are_empty_for_a_file_outside_the_repository() {
    let (_dir, repository) = repository_with(&[("a.pn", "[b]\n"), ("b.pn", "")]);
    let (_other, outside) = notes_dir();
    std::fs::write(outside.join("b.pn"), "").unwrap();

    assert!(repository
        .calculate_back_links(&outside.join("b.pn"))
        .is_empty());
}

#[tokio::test]
async fn two_hop_links_group_other_notes_by_the_shared_target() {
    let (_dir, repository) = repository_with(&[
        ("a.pn", "[hub]\n[lonely]\n"),
        ("b.pn", "[hub]\n"),
        ("c.pn", "[hub]\n"),
        ("hub.pn", ""),
        ("lonely.pn", ""),
    ]);

    let two_hop = repository
        .calculate_two_hop_links(&repository.root_dir.join("a.pn"))
        .await;

    assert_eq!(
        two_hop,
        vec![("hub".to_string(), vec!["b".to_string(), "c".to_string()])]
    );
}

#[test]
fn rewriting_a_file_drops_the_links_it_no_longer_has() {
    let (_dir, repository) = repository_with(&[("a.pn", "[b]\n"), ("b.pn", "")]);
    let b = repository.root_dir.join("b.pn");
    assert_eq!(repository.count_back_links(&b), 1);

    repository.add_file_to_graph(&repository.root_dir.join("a.pn"), "no links now\n");

    assert_eq!(repository.count_back_links(&b), 0);
}

#[test]
fn a_link_to_a_missing_note_still_counts_as_a_back_link_once_it_exists() {
    let (_dir, repository) = repository_with(&[("a.pn", "[later]\n")]);
    let later = repository.root_dir.join("later.pn");
    std::fs::write(&later, "").unwrap();

    assert_eq!(repository.count_back_links(&later), 1);
}

#[test]
fn file_metadata_carries_the_incoming_link_count() {
    let (_dir, repository) = repository_with(&[
        ("a.pn", "[b]\n[b#x]\n"),
        ("c.pn", "[b]\n"),
        ("b.pn", "target\n"),
    ]);

    let mut files = Vec::new();
    let mut metadata = HashMap::new();
    repository.collect_patto_files_with_metadata(&repository.root_dir, &mut files, &mut metadata);

    files.sort();
    assert_eq!(files, ["a.pn", "b.pn", "c.pn"]);
    assert_eq!(metadata["b.pn"].link_count, 3);
    assert_eq!(metadata["a.pn"].link_count, 0);
}

#[test]
fn link_names_map_to_note_paths_and_back() {
    let (_dir, repository) = repository_with(&[("note.pn", "")]);
    let root = repository.root_dir.clone();

    assert_eq!(repository.link_to_path("note"), Some(root.join("note.pn")));
    assert_eq!(repository.link_to_path("missing"), None);
    assert_eq!(repository.link_to_path(""), None);
    assert_eq!(
        repository.path_to_link(&root.join("sub").join("deep.pn")),
        Some("deep".to_string())
    );
    assert_eq!(
        repository.path_to_link(&PathBuf::from("/elsewhere/x.pn")),
        None
    );
}

#[test]
fn link_uris_are_percent_encoded_under_the_root() {
    let (_dir, repository) = repository_with(&[]);
    let root_uri = Url::from_directory_path(&repository.root_dir).unwrap();
    let mut root_without_slash = root_uri.clone();
    root_without_slash.set_path(root_uri.path().trim_end_matches('/'));

    let uri = repository.link_to_uri("sp ace", &root_uri).unwrap();
    assert!(uri.path().ends_with("/sp%20ace.pn"), "{uri}");
    assert_eq!(
        repository
            .link_to_uri("sp ace", &root_without_slash)
            .unwrap(),
        uri
    );
    assert_eq!(repository.link_to_uri("", &root_uri), None);
}

#[test]
fn percent_escapes_are_normalised_to_upper_case() {
    let url = Url::parse("file:///notes/caf%c3%a9.pn").unwrap();
    assert_eq!(
        Repository::normalize_url_percent_encoding(&url).as_str(),
        "file:///notes/caf%C3%A9.pn"
    );
}

#[test]
fn pinning_saves_the_workspace_config_and_broadcasts_it() {
    let (_dir, repository) = repository_with(&[("a.pn", "")]);
    let mut rx = repository.subscribe();

    repository.pin_file("a.pn").unwrap();
    repository.pin_file("a.pn").unwrap();

    let saved = load_workspace_config(&repository.root_dir);
    assert_eq!(saved.pinned_files, ["a.pn"]);
    let RepositoryMessage::WorkspaceConfigChanged(config) = rx.try_recv().unwrap() else {
        panic!("expected a WorkspaceConfigChanged message");
    };
    assert_eq!(config.pinned_files, ["a.pn"]);

    repository.unpin_file("a.pn").unwrap();
    assert!(load_workspace_config(&repository.root_dir)
        .pinned_files
        .is_empty());
}
