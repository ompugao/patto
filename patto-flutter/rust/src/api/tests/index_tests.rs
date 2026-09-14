use super::Workspace;
use crate::api::error::PattoError;
use crate::api::index::*;

fn build(ws: &Workspace) -> IndexStats {
    index_build(ws.root(), |_| {}).unwrap()
}

#[test]
fn backlinks_list_every_note_that_points_here() {
    let ws = Workspace::new();
    ws.write("a.pn", "see [b] for details\n");
    ws.write("c.pn", "also [b#sec]\n");
    ws.write("b.pn", "the target\n");
    build(&ws);

    let links = backlinks(ws.root(), "b.pn".to_string()).unwrap();
    let sources: Vec<&str> = links.iter().map(|l| l.source_name.as_str()).collect();
    assert_eq!(sources, vec!["a", "c"]);
    assert_eq!(links[0].context, "see [b] for details");
    assert_eq!(links[1].target_anchor.as_deref(), Some("sec"));
}

#[test]
fn a_note_with_no_backlinks_gets_an_empty_list() {
    let ws = Workspace::new();
    ws.write("lonely.pn", "nothing points here\n");
    build(&ws);
    assert!(backlinks(ws.root(), "lonely.pn".to_string())
        .unwrap()
        .is_empty());
}

#[test]
fn two_hop_links_group_by_the_shared_note() {
    let ws = Workspace::new();
    ws.write("a.pn", "[hub]\n");
    ws.write("c.pn", "[hub]\n");
    ws.write("d.pn", "[hub]\n");
    ws.write("hub.pn", "central\n");
    build(&ws);

    let hops = two_hop_links(ws.root(), "a.pn".to_string()).unwrap();
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0].via_name, "hub");
    assert_eq!(hops[0].names, vec!["c".to_string(), "d".to_string()]);
}

#[test]
fn two_hop_links_work_through_a_note_that_does_not_exist_yet() {
    let ws = Workspace::new();
    ws.write("a.pn", "[planned]\n");
    ws.write("b.pn", "[planned]\n");
    build(&ws);

    let hops = two_hop_links(ws.root(), "a.pn".to_string()).unwrap();
    assert_eq!(hops[0].via_name, "planned");
    assert_eq!(hops[0].names, vec!["b".to_string()]);
}

#[test]
fn two_hop_groups_are_ordered_by_size() {
    let ws = Workspace::new();
    ws.write("a.pn", "[small] [big]\n");
    ws.write("s1.pn", "[small]\n");
    ws.write("b1.pn", "[big]\n");
    ws.write("b2.pn", "[big]\n");
    build(&ws);

    let hops = two_hop_links(ws.root(), "a.pn".to_string()).unwrap();
    assert_eq!(hops[0].via_name, "big");
    assert_eq!(hops[1].via_name, "small");
}

#[test]
fn updating_a_note_updates_its_backlinks() {
    let ws = Workspace::new();
    ws.write("a.pn", "[b]\n");
    ws.write("b.pn", "target\n");
    build(&ws);
    assert_eq!(backlinks(ws.root(), "b.pn".to_string()).unwrap().len(), 1);

    ws.write("a.pn", "no links any more\n");
    index_update_file(ws.root(), "a.pn".to_string()).unwrap();
    assert!(backlinks(ws.root(), "b.pn".to_string()).unwrap().is_empty());
}

#[test]
fn deleting_a_note_drops_it_from_the_index() {
    let ws = Workspace::new();
    ws.write("a.pn", "[b]\n");
    ws.write("b.pn", "target\n");
    build(&ws);

    std::fs::remove_file(ws.path().join("a.pn")).unwrap();
    index_update_file(ws.root(), "a.pn".to_string()).unwrap();

    assert!(backlinks(ws.root(), "b.pn".to_string()).unwrap().is_empty());
    assert!(!link_counts(ws.root())
        .unwrap()
        .iter()
        .any(|c| c.name == "a"));
}

#[test]
fn refresh_picks_up_new_and_removed_notes() {
    let ws = Workspace::new();
    ws.write("a.pn", "[b]\n");
    ws.write("b.pn", "target\n");
    build(&ws);

    ws.write("c.pn", "[b]\n");
    std::fs::remove_file(ws.path().join("a.pn")).unwrap();
    let stats = index_refresh(ws.root()).unwrap();

    assert_eq!(stats.notes, 2);
    let sources: Vec<String> = backlinks(ws.root(), "b.pn".to_string())
        .unwrap()
        .into_iter()
        .map(|l| l.source_name)
        .collect();
    assert_eq!(sources, vec!["c".to_string()]);
}

#[test]
fn link_counts_rank_the_most_linked_note_first() {
    let ws = Workspace::new();
    ws.write("a.pn", "[hub]\n");
    ws.write("b.pn", "[hub]\n");
    ws.write("hub.pn", "[a]\n");
    build(&ws);

    let counts = link_counts(ws.root()).unwrap();
    assert_eq!(counts[0].name, "hub");
    assert_eq!(counts[0].backlinks, 2);
    assert_eq!(counts[0].outgoing, 1);
}

#[test]
fn subdirectory_notes_are_addressed_by_their_full_name() {
    let ws = Workspace::new();
    ws.write("a.pn", "[dir/deep]\n");
    ws.write("dir/deep.pn", "target\n");
    build(&ws);

    let links = backlinks(ws.root(), "dir/deep.pn".to_string()).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].source_name, "a");
}

#[test]
fn self_links_are_not_indexed_as_outgoing_links() {
    let ws = Workspace::new();
    ws.write("a.pn", "jump [#here]\nthe spot  #here\n");
    build(&ws);
    assert_eq!(link_counts(ws.root()).unwrap()[0].outgoing, 0);
}

#[test]
fn queries_before_the_index_is_built_say_so() {
    let ws = Workspace::new();
    ws.write("a.pn", "x\n");
    assert!(matches!(
        backlinks(ws.root(), "a.pn".to_string()),
        Err(PattoError::IndexNotBuilt(_))
    ));
}

#[test]
fn build_reports_progress_and_finishes() {
    let ws = Workspace::new();
    for i in 0..45 {
        ws.write(&format!("n{i}.pn"), "x\n");
    }

    let seen = std::sync::Mutex::new(Vec::new());
    let stats = index_build(ws.root(), |p| seen.lock().unwrap().push(p)).unwrap();

    assert_eq!(stats.notes, 45);
    let seen = seen.into_inner().unwrap();
    assert!(seen.iter().any(|p| p.done));
    assert_eq!(seen.last().unwrap().scanned, 45);
}
