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
fn text_search_matches_contents_case_insensitively() {
    let ws = Workspace::new();
    ws.write("a.pn", "first line\n\tThe Quick fox\nlast");
    ws.write("b.pn", "nothing here");

    let hits = search_text(ws.root(), "quick".to_string(), 10, 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].note.name, "a");
    assert!(!hits[0].name_matches);
    assert_eq!(hits[0].total_matches, 1);
    assert_eq!(hits[0].matches[0].row, 1);
    assert_eq!(hits[0].matches[0].line, "The Quick fox");
}

#[test]
fn text_search_caps_lines_per_note_but_counts_them_all() {
    let ws = Workspace::new();
    ws.write("many.pn", "hit\nhit\nmiss\nhit\nhit");
    ws.write("one.pn", "hit");

    let hits = search_text(ws.root(), "hit".to_string(), 10, 2).unwrap();
    assert_eq!(hits[0].note.name, "many");
    assert_eq!(hits[0].total_matches, 4);
    let rows: Vec<u32> = hits[0].matches.iter().map(|m| m.row).collect();
    assert_eq!(rows, vec![0, 1]);

    assert_eq!(
        search_text(ws.root(), "hit".to_string(), 1, 2)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn text_search_includes_name_matches_first() {
    let ws = Workspace::new();
    ws.write("groceries.pn", "milk");
    ws.write("other.pn", "groceries groceries\ngroceries");

    let hits = search_text(ws.root(), "Grocer".to_string(), 10, 5).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].note.name, "groceries");
    assert!(hits[0].name_matches);
    assert!(hits[0].matches.is_empty());
    assert_eq!(hits[1].total_matches, 2);
}

#[test]
fn text_search_ignores_empty_queries_and_git_internals() {
    let ws = Workspace::new();
    ws.write("a.pn", "secret");
    ws.write(".git/objects/b.pn", "secret");

    assert!(search_text(ws.root(), "  ".to_string(), 10, 5)
        .unwrap()
        .is_empty());
    let hits = search_text(ws.root(), "secret".to_string(), 10, 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].note.name, "a");
}

#[test]
fn text_search_clips_long_lines_around_the_match() {
    let ws = Workspace::new();
    let line = format!("{}目的の言葉{}", "あ".repeat(300), "い".repeat(300));
    ws.write("long.pn", &line);

    let hits = search_text(ws.root(), "目的".to_string(), 10, 5).unwrap();
    let snippet = &hits[0].matches[0].line;
    assert!(snippet.contains("目的の言葉"));
    assert!(snippet.starts_with('…') && snippet.ends_with('…'));
    assert!(snippet.chars().count() <= 162);
}

#[test]
fn deleting_a_missing_note_reports_not_found() {
    let ws = Workspace::new();
    assert!(matches!(
        delete_note(ws.root(), "nope.pn".to_string()),
        Err(PattoError::NotFound(_))
    ));
}

#[test]
fn appending_creates_a_missing_note() {
    let ws = Workspace::new();
    let meta = append_to_note(ws.root(), "Inbox".to_string(), "first".to_string()).unwrap();
    assert_eq!(meta.rel_path, "Inbox.pn");
    assert_eq!(ws.read("Inbox.pn"), "first\n");
}

#[test]
fn appending_starts_on_a_new_line() {
    let ws = Workspace::new();
    ws.write("a.pn", "one");
    append_to_note(ws.root(), "a".to_string(), "two".to_string()).unwrap();
    assert_eq!(ws.read("a.pn"), "one\ntwo\n");

    append_to_note(ws.root(), "a".to_string(), "three".to_string()).unwrap();
    assert_eq!(ws.read("a.pn"), "one\ntwo\nthree\n");
}

#[test]
fn appending_keeps_nesting_and_ends_with_one_newline() {
    let ws = Workspace::new();
    ws.write("a.pn", "one\n");
    append_to_note(ws.root(), "a".to_string(), "head\n\tchild\n\n".to_string()).unwrap();
    assert_eq!(ws.read("a.pn"), "one\nhead\n\tchild\n");
}

#[test]
fn appending_normalises_line_endings() {
    let ws = Workspace::new();
    ws.write("a.pn", "one\r\ntwo");
    append_to_note(ws.root(), "a".to_string(), "x\r\ny".to_string()).unwrap();
    assert_eq!(ws.read("a.pn"), "one\ntwo\nx\ny\n");
}

#[test]
fn appending_to_a_bad_name_is_rejected() {
    let ws = Workspace::new();
    assert!(matches!(
        append_to_note(ws.root(), "../evil".to_string(), "x".to_string()),
        Err(PattoError::InvalidName(_))
    ));
}
