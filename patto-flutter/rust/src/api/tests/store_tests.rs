use super::Workspace;
use crate::api::error::PattoError;
use crate::api::store::*;

#[test]
fn names_map_to_paths_and_back() {
    assert_eq!(name_to_rel_path("note").unwrap(), "note.pn");
    assert_eq!(name_to_rel_path("dir/note").unwrap(), "dir/note.pn");
    assert_eq!(rel_path_to_name("dir/note.pn"), "dir/note");
    assert_eq!(rel_path_to_name("note.pn"), "note");
}

#[test]
fn names_that_escape_the_root_are_rejected() {
    for bad in ["", "../evil", "/abs", ".git/config", "a/../b"] {
        assert!(
            matches!(name_to_rel_path(bad), Err(PattoError::InvalidName(_))),
            "expected {bad:?} to be rejected"
        );
    }
}

#[test]
fn paths_that_escape_the_root_are_rejected() {
    assert!(resolve("/tmp/notes", "../outside.pn").is_err());
    assert!(resolve("/tmp/notes", "/etc/passwd").is_err());
    assert!(resolve("/tmp/notes", "dir/ok.pn").is_ok());
}

#[test]
fn listing_finds_notes_in_subdirectories_and_skips_git() {
    let ws = Workspace::new();
    ws.write("a.pn", "a");
    ws.write("dir/b.pn", "b");
    ws.write("notes.txt", "not a note");
    ws.write(".git/config", "[core]");
    ws.write(".git/objects/c.pn", "hidden");

    let names: Vec<String> = list_notes(ws.root())
        .unwrap()
        .into_iter()
        .map(|n| n.name)
        .collect();

    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"dir/b".to_string()));
}

#[test]
fn writing_then_reading_round_trips() {
    let ws = Workspace::new();
    write_note(ws.root(), "x.pn".to_string(), "hello".to_string()).unwrap();
    assert_eq!(read_note(ws.root(), "x.pn".to_string()).unwrap(), "hello");
}

#[test]
fn writing_leaves_no_temporary_file_behind() {
    let ws = Workspace::new();
    write_note(ws.root(), "x.pn".to_string(), "hello".to_string()).unwrap();
    let leftovers: Vec<_> = std::fs::read_dir(ws.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("tmp"))
        .collect();
    assert!(leftovers.is_empty(), "found {leftovers:?}");
}

#[test]
fn creating_a_note_twice_fails() {
    let ws = Workspace::new();
    create_note(ws.root(), "new".to_string(), String::new()).unwrap();
    assert!(matches!(
        create_note(ws.root(), "new".to_string(), String::new()),
        Err(PattoError::AlreadyExists(_))
    ));
}

#[test]
fn wiki_links_resolve_only_to_notes_that_exist() {
    let ws = Workspace::new();
    ws.write("target.pn", "x");
    assert_eq!(
        resolve_wiki_link(ws.root(), "target".to_string()).unwrap(),
        Some("target.pn".to_string())
    );
    assert_eq!(
        resolve_wiki_link(ws.root(), "missing".to_string()).unwrap(),
        None
    );
}

#[test]
fn search_ranks_fuzzy_matches_and_an_empty_query_lists_everything() {
    let ws = Workspace::new();
    ws.write("meeting notes.pn", "x");
    ws.write("meeting.pn", "x");
    ws.write("unrelated.pn", "x");

    let hits: Vec<String> = search_notes(ws.root(), "meet".to_string(), 10)
        .unwrap()
        .into_iter()
        .map(|n| n.name)
        .collect();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|h| h.contains("meeting")));

    assert_eq!(search_notes(ws.root(), String::new(), 10).unwrap().len(), 3);
    assert_eq!(search_notes(ws.root(), String::new(), 2).unwrap().len(), 2);
}

#[test]
fn deleting_a_missing_note_reports_not_found() {
    let ws = Workspace::new();
    assert!(matches!(
        delete_note(ws.root(), "nope.pn".to_string()),
        Err(PattoError::NotFound(_))
    ));
}
