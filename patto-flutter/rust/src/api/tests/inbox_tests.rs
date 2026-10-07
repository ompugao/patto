use super::Workspace;
use crate::api::error::PattoError;
use crate::api::inbox::*;

const SAMPLE: &str = "2026-10-07\n\t09:12 bought coffee beans\n\t11:40 idea for the parser\n\t\tcontinuation line\n2026-10-08\n\t08:05 [https://example.com Title]\n";

fn append(ws: &Workspace, date: &str, time: &str, text: &str) {
    inbox_append(
        ws.root(),
        "Inbox".to_string(),
        date.to_string(),
        time.to_string(),
        text.to_string(),
    )
    .unwrap();
}

#[test]
fn a_missing_note_has_no_posts() {
    let ws = Workspace::new();
    assert!(inbox_posts(ws.root(), "Inbox".to_string())
        .unwrap()
        .is_empty());
    ws.write("Inbox.pn", "");
    assert!(inbox_posts(ws.root(), "Inbox".to_string())
        .unwrap()
        .is_empty());
}

#[test]
fn posts_are_parsed_oldest_first_with_their_lines() {
    let posts = parse_posts(SAMPLE);
    assert_eq!(posts.len(), 3);
    assert_eq!(posts[0].date, "2026-10-07");
    assert_eq!(posts[0].time, "09:12");
    assert_eq!(posts[0].text, "bought coffee beans");
    assert_eq!(posts[0].line, 1);
    assert_eq!(posts[1].body, vec!["continuation line".to_string()]);
    assert_eq!(posts[1].line, 2);
    assert_eq!(posts[2].date, "2026-10-08");
    assert_eq!(posts[2].text, "[https://example.com Title]");
    assert_eq!(posts[2].line, 5);
}

#[test]
fn appending_creates_the_note_with_a_heading() {
    let ws = Workspace::new();
    let meta = inbox_append(
        ws.root(),
        "Inbox".to_string(),
        "2026-10-07".to_string(),
        "09:12".to_string(),
        "bought coffee beans".to_string(),
    )
    .unwrap();
    assert_eq!(meta.rel_path, "Inbox.pn");
    assert_eq!(
        ws.read("Inbox.pn"),
        "2026-10-07\n\t09:12 bought coffee beans\n"
    );
}

#[test]
fn appending_nests_under_todays_heading() {
    let ws = Workspace::new();
    append(&ws, "2026-10-07", "09:12", "bought coffee beans");
    append(
        &ws,
        "2026-10-07",
        "11:40",
        "idea for the parser\ncontinuation line",
    );
    append(&ws, "2026-10-08", "08:05", "[https://example.com Title]");
    assert_eq!(ws.read("Inbox.pn"), SAMPLE);
}

#[test]
fn appending_after_other_content_adds_a_heading() {
    let ws = Workspace::new();
    ws.write(
        "Inbox.pn",
        "2026-10-07\n\t09:12 one\nsome hand-written line",
    );
    append(&ws, "2026-10-07", "10:00", "two");
    assert_eq!(
        ws.read("Inbox.pn"),
        "2026-10-07\n\t09:12 one\nsome hand-written line\n2026-10-07\n\t10:00 two\n"
    );
}

#[test]
fn appending_to_a_note_without_a_trailing_newline() {
    let ws = Workspace::new();
    ws.write("Inbox.pn", "2026-10-07\n\t09:12 one");
    append(&ws, "2026-10-07", "10:00", "two");
    assert_eq!(
        ws.read("Inbox.pn"),
        "2026-10-07\n\t09:12 one\n\t10:00 two\n"
    );
}

#[test]
fn posts_drop_blank_lines_and_normalise_endings() {
    assert_eq!(
        format_post("10:00", "\tfirst  \r\n\r\n\tsecond\r\n   \n"),
        "\t10:00 first\n\t\t\tsecond"
    );
    assert_eq!(format_post("10:00", " \n\t\n"), "");
}

#[test]
fn an_empty_post_is_refused() {
    let ws = Workspace::new();
    let err = inbox_append(
        ws.root(),
        "Inbox".to_string(),
        "2026-10-07".to_string(),
        "10:00".to_string(),
        "\n".to_string(),
    )
    .unwrap_err();
    assert!(matches!(err, PattoError::Io(_)));
    assert!(!ws.path().join("Inbox.pn").exists());
}

#[test]
fn hand_edited_lines_are_tolerated() {
    let content = "intro line\n2026-10-07\n\tnot a post\n\t09:12 post\n\t\tbody\n\t\t\tdeeper body\n\tnote\n\t\tnot body\n2026-10-08 trailing\n\t10:00 orphan\n2026-10-09\n\t11:00 last\r\n";
    let posts = parse_posts(content);
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].text, "post");
    assert_eq!(
        posts[0].body,
        vec!["body".to_string(), "\tdeeper body".to_string()]
    );
    assert_eq!(posts[1].date, "2026-10-09");
    assert_eq!(posts[1].text, "last");
}

#[test]
fn a_round_trip_keeps_the_text() {
    let ws = Workspace::new();
    append(&ws, "2026-10-07", "09:12", "head\n  body one\nbody two");
    let posts = inbox_posts(ws.root(), "Inbox".to_string()).unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].text, "head");
    assert_eq!(
        posts[0].body,
        vec!["  body one".to_string(), "body two".to_string()]
    );
}

#[test]
fn a_bad_name_is_rejected() {
    let ws = Workspace::new();
    assert!(matches!(
        inbox_posts(ws.root(), "../evil".to_string()),
        Err(PattoError::InvalidName(_))
    ));
}
