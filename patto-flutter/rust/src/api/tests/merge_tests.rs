use crate::api::merge::*;

fn lines(s: &[&str]) -> Vec<String> {
    s.iter().map(|l| l.to_string()).collect()
}

#[test]
fn edits_on_different_lines_do_not_conflict() {
    let base = "a\nb\nc\nd\n";
    let ours = "a\nB\nc\nd\n";
    let theirs = "a\nb\nc\nD\n";
    let merged = merge_lines(base, ours, theirs).unwrap();

    assert_eq!(merged.conflict_count(), 0);
    assert_eq!(
        merged.regions,
        vec![
            MergeRegion::Unchanged {
                lines: lines(&["a"])
            },
            MergeRegion::Ours {
                base: lines(&["b"]),
                lines: lines(&["B"]),
            },
            MergeRegion::Unchanged {
                lines: lines(&["c"])
            },
            MergeRegion::Theirs {
                base: lines(&["d"]),
                lines: lines(&["D"]),
            },
        ]
    );
    assert_eq!(assemble(&merged, &[]), "a\nB\nc\nD\n");
}

#[test]
fn edits_to_the_same_line_conflict() {
    let merged = merge_lines("a\nmilk\nc\n", "a\nsoy milk\nc\n", "a\noat milk\nc\n").unwrap();

    assert_eq!(merged.conflict_count(), 1);
    assert_eq!(
        merged.regions[1],
        MergeRegion::Conflict {
            base: lines(&["milk"]),
            ours: lines(&["soy milk"]),
            theirs: lines(&["oat milk"]),
            suggestion: None,
        }
    );
    assert_eq!(
        assemble(&merged, &[lines(&["oat milk"])]),
        "a\noat milk\nc\n"
    );
}

#[test]
fn identical_changes_are_not_a_conflict() {
    let merged = merge_lines("a\nb\n", "a\nx\n", "a\nx\n").unwrap();
    assert_eq!(merged.conflict_count(), 0);
    assert!(matches!(merged.regions[1], MergeRegion::Same { .. }));
}

#[test]
fn a_property_change_and_a_text_change_combine() {
    let base = "\tmilk {@task status=todo}\n";
    let ours = "\tmilk {@task status=done}\n";
    let theirs = "\toat milk {@task status=todo}\n";
    let merged = merge_lines(base, ours, theirs).unwrap();

    let MergeRegion::Conflict { suggestion, .. } = &merged.regions[0] else {
        panic!("expected a conflict, got {:?}", merged.regions);
    };
    assert_eq!(
        suggestion,
        &Some(Suggestion {
            kind: SuggestionKind::Combined,
            lines: lines(&["\toat milk {@task status=done}"]),
        })
    );
}

#[test]
fn a_new_trailing_property_combines_with_a_text_change() {
    let merged = merge_lines("milk\n", "milk {@task status=todo}\n", "oat milk\n").unwrap();
    let MergeRegion::Conflict { suggestion, .. } = &merged.regions[0] else {
        panic!("expected a conflict");
    };
    assert_eq!(
        suggestion.as_ref().map(|s| s.lines.clone()),
        Some(lines(&["oat milk {@task status=todo}"]))
    );
}

#[test]
fn two_text_changes_have_no_suggestion() {
    let merged = merge_lines("milk {@task}\n", "soy milk {@task}\n", "oat milk {@task}\n").unwrap();
    let MergeRegion::Conflict { suggestion, .. } = &merged.regions[0] else {
        panic!("expected a conflict");
    };
    assert_eq!(suggestion, &None);
}

#[test]
fn additions_at_the_same_place_suggest_both() {
    let merged = merge_lines("a\nz\n", "a\nb\nz\n", "a\nc\nz\n").unwrap();
    assert_eq!(merged.conflict_count(), 1);
    let MergeRegion::Conflict { suggestion, .. } = &merged.regions[1] else {
        panic!("expected a conflict");
    };
    assert_eq!(
        suggestion,
        &Some(Suggestion {
            kind: SuggestionKind::Both,
            lines: lines(&["b", "c"]),
        })
    );
}

#[test]
fn adjacent_line_edits_do_not_conflict() {
    let merged = merge_lines("a\nb\nc\n", "A\nb\nc\n", "a\nB\nc\n").unwrap();
    assert_eq!(merged.conflict_count(), 0);
    assert_eq!(assemble(&merged, &[]), "A\nB\nc\n");
}

#[test]
fn a_missing_final_newline_is_kept_from_our_side() {
    let merged = merge_lines("a\nb", "a\nB", "a\nb").unwrap();
    assert!(!merged.trailing_newline);
    assert_eq!(assemble(&merged, &[]), "a\nB");
}

#[test]
fn a_deleted_side_shows_everything_as_changed() {
    let merged = merge_lines("a\nb\n", "", "a\nB\n").unwrap();
    assert_eq!(merged.conflict_count(), 1);
    assert_eq!(merged.changed_lines(), (2, 2));
}
