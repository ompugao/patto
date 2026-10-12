use chrono::NaiveDate;
use patto::ast_query::{
    conceal_urls, find_anchor, gather_completed_tasks, gather_tasks, gather_wikilinks, task_label,
    walk_lines,
};
use patto::parser::{self, AstNode, Deadline};

fn parse(text: &str) -> AstNode {
    let result = parser::parse_text(text);
    assert!(
        result.parse_errors.is_empty(),
        "unexpected parse errors: {:?}",
        result.parse_errors
    );
    result.ast
}

fn first_line(text: &str) -> AstNode {
    parse(text).children()[0].clone()
}

fn nested_line(text: &str) -> AstNode {
    first_line(&format!("parent\n\t{text}")).children()[0].clone()
}

#[test]
fn task_label_strips_the_task_token_wherever_it_sits() {
    assert_eq!(
        task_label(&first_line("buy milk {@task status=todo due=2026-06-01}")),
        "buy milk"
    );
    assert_eq!(
        task_label(&first_line("{@task status=todo} buy milk")),
        "buy milk"
    );
    assert_eq!(
        task_label(&first_line("buy {@task status=todo} milk")),
        "buy milk"
    );
    assert_eq!(task_label(&nested_line("-2026-06-01")), "");
}

#[test]
fn task_label_of_a_line_without_a_task_is_its_trimmed_text() {
    assert_eq!(task_label(&nested_line("plain line")), "plain line");
}

#[test]
fn conceal_urls_replaces_the_url_part_of_a_link_with_a_glyph() {
    assert_eq!(
        conceal_urls("see [https://example.com docs] now"),
        "see [🔗docs] now"
    );
    assert_eq!(
        conceal_urls("see [docs https://example.com] now"),
        "see [docs🔗] now"
    );
    assert_eq!(conceal_urls("[wiki page]"), "[wiki page]");
}

#[test]
fn find_anchor_returns_the_nested_line_that_carries_it() {
    let root = parse("top\n\tmiddle #here\n\t\tbottom\n");
    let found = find_anchor(&root, "here").expect("anchor should be found");
    assert_eq!(found.extract_str(), "\tmiddle #here");
    assert!(find_anchor(&root, "missing").is_none());
}

#[test]
fn walk_lines_reports_each_line_with_its_nesting_depth() {
    let root = parse("a\n\tb\n\t\tc\nd\n");
    let mut seen = Vec::new();
    walk_lines(&root, &mut |line, depth| {
        seen.push((line.extract_str().trim().to_string(), depth));
    });
    assert_eq!(
        seen,
        [
            ("a".to_string(), 0),
            ("b".to_string(), 1),
            ("c".to_string(), 2),
            ("d".to_string(), 0),
        ]
    );
}

#[test]
fn gather_tasks_skips_done_tasks_and_keeps_the_due_date() {
    let root = parse(
        "open {@task status=todo due=2026-06-01}\n\tclosed -2026-05-01\n\tactive *2026-05-02\n",
    );
    let mut tasks = Vec::new();
    gather_tasks(&root, &mut tasks);
    let labels: Vec<_> = tasks
        .iter()
        .map(|(line, due)| (task_label(line), due.clone()))
        .collect();
    let date = |s| Deadline::Date(NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap());
    assert_eq!(
        labels,
        [
            ("open".to_string(), date("2026-06-01")),
            ("active".to_string(), date("2026-05-02")),
        ]
    );
}

#[test]
fn gather_completed_tasks_requires_a_completed_at_date() {
    let root = parse(
        "a {@task status=done completed_at=2026-05-01}\nb {@task status=done completed_at=2026-05-02T10:00}\nc {@task status=done}\nd {@task status=done completed_at=sometime}\ne {@task status=todo completed_at=2026-05-03}\n",
    );
    let mut done = Vec::new();
    gather_completed_tasks(&root, &mut done);
    let found: Vec<_> = done
        .iter()
        .map(|(line, date)| (task_label(line), *date))
        .collect();
    let date = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
    assert_eq!(
        found,
        [
            ("a".to_string(), date("2026-05-01")),
            ("b".to_string(), date("2026-05-02")),
        ]
    );
}

#[test]
fn gather_wikilinks_finds_links_inside_decorations_and_nested_lines() {
    let root = parse("[* [bold page]]\n\t[nested page#part]\n");
    let mut links = Vec::new();
    gather_wikilinks(&root, &mut links);
    let found: Vec<_> = links
        .iter()
        .map(|(link, anchor, location)| (link.as_str(), anchor.as_deref(), location.row))
        .collect();
    assert_eq!(
        found,
        [("bold page", None, 0), ("nested page", Some("part"), 1)]
    );
}
