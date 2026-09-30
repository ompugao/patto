//! Three-way, line-level merge of one note.
//!
//! A paused sync leaves three versions of a note: the last common one (base),
//! the phone's (ours) and the remote's (theirs). This splits them into regions
//! the app can show one after another: lines nobody touched, lines only one
//! side changed, and conflicts where both did. Notes are line-oriented, so lines
//! are the unit throughout.
//!
//! The line diffs come from libgit2, which already ships xdiff.

use git2::{DiffOptions, Patch};

use crate::api::error::PattoResult;

/// How a suggested resolution was arrived at, so the app can label it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    /// Each side changed a different part of the same lines: one the text, the
    /// other the `{@...}` properties.
    Combined,
    /// Both sides only added lines at the same place: keep both, ours first.
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeRegion {
    Unchanged {
        lines: Vec<String>,
    },
    /// Only the phone changed these lines.
    Ours {
        base: Vec<String>,
        lines: Vec<String>,
    },
    /// Only the remote changed these lines.
    Theirs {
        base: Vec<String>,
        lines: Vec<String>,
    },
    /// Both sides made the same change.
    Same {
        base: Vec<String>,
        lines: Vec<String>,
    },
    Conflict {
        base: Vec<String>,
        ours: Vec<String>,
        theirs: Vec<String>,
        suggestion: Option<Suggestion>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedNote {
    pub regions: Vec<MergeRegion>,
    /// Whether the merged text should end with a newline.
    pub trailing_newline: bool,
}

impl MergedNote {
    pub fn conflict_count(&self) -> usize {
        self.regions
            .iter()
            .filter(|r| matches!(r, MergeRegion::Conflict { .. }))
            .count()
    }

    /// Lines changed by the phone and by the remote, roughly: each changed
    /// region counts as many lines as its longer side.
    pub fn changed_lines(&self) -> (u32, u32) {
        let (mut ours, mut theirs) = (0usize, 0usize);
        for region in &self.regions {
            match region {
                MergeRegion::Unchanged { .. } => {}
                MergeRegion::Ours { base, lines } => ours += base.len().max(lines.len()),
                MergeRegion::Theirs { base, lines } => theirs += base.len().max(lines.len()),
                MergeRegion::Same { base, lines } => {
                    let n = base.len().max(lines.len());
                    ours += n;
                    theirs += n;
                }
                MergeRegion::Conflict {
                    base,
                    ours: o,
                    theirs: t,
                    ..
                } => {
                    ours += base.len().max(o.len());
                    theirs += base.len().max(t.len());
                }
            }
        }
        (ours as u32, theirs as u32)
    }
}

/// Split into lines without their terminators. A final newline does not start
/// another line, matching how git counts.
fn split_lines(text: &str) -> Vec<String> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return Vec::new();
    }
    body.split('\n').map(str::to_string).collect()
}

/// For each line of `old`, the line of `new` it is kept as, if any.
fn line_map(old: &[String], new: &[String]) -> PattoResult<Vec<Option<usize>>> {
    // Diff normalised text so a missing final newline is not a change.
    let join = |lines: &[String]| {
        let mut s = lines.join("\n");
        if !lines.is_empty() {
            s.push('\n');
        }
        s
    };
    let (old_text, new_text) = (join(old), join(new));

    let mut opts = DiffOptions::new();
    opts.context_lines(0).patience(true);
    let patch = Patch::from_buffers(
        old_text.as_bytes(),
        None,
        new_text.as_bytes(),
        None,
        Some(&mut opts),
    )?;

    let mut map = vec![None; old.len()];
    // Walk the gaps between hunks, where lines are kept one for one.
    let (mut o, mut n) = (0usize, 0usize);
    for h in 0..patch.num_hunks() {
        let (hunk, _) = patch.hunk(h)?;
        // Hunk starts are 1-based, except that an empty side names the line
        // before the change.
        let start = |line: u32, count: u32| {
            if count == 0 {
                line as usize
            } else {
                line as usize - 1
            }
        };
        let old_start = start(hunk.old_start(), hunk.old_lines());
        let new_start = start(hunk.new_start(), hunk.new_lines());
        while o < old_start {
            map[o] = Some(n);
            o += 1;
            n += 1;
        }
        debug_assert_eq!(n, new_start);
        o += hunk.old_lines() as usize;
        n = new_start + hunk.new_lines() as usize;
    }
    while o < old.len() {
        map[o] = Some(n);
        o += 1;
        n += 1;
    }
    Ok(map)
}

/// Merge three versions of a note into regions.
pub fn merge_lines(base: &str, ours: &str, theirs: &str) -> PattoResult<MergedNote> {
    let base_lines = split_lines(base);
    let ours_lines = split_lines(ours);
    let theirs_lines = split_lines(theirs);
    let to_ours = line_map(&base_lines, &ours_lines)?;
    let to_theirs = line_map(&base_lines, &theirs_lines)?;

    let mut regions = Vec::new();
    let (mut i, mut a, mut b) = (0usize, 0usize, 0usize);
    let n = base_lines.len();

    loop {
        // Lines kept by both sides.
        let start = i;
        while i < n && to_ours[i] == Some(a) && to_theirs[i] == Some(b) {
            i += 1;
            a += 1;
            b += 1;
        }
        if i > start {
            push_region(
                &mut regions,
                MergeRegion::Unchanged {
                    lines: base_lines[start..i].to_vec(),
                },
            );
        }
        if i == n && a == ours_lines.len() && b == theirs_lines.len() {
            break;
        }

        // Everything up to the next line both sides kept is one changed region.
        let (j, next_a, next_b) = (i..n)
            .find_map(|j| Some((j, to_ours[j]?, to_theirs[j]?)))
            .unwrap_or((n, ours_lines.len(), theirs_lines.len()));

        push_changed(
            &mut regions,
            &base_lines[i..j],
            &ours_lines[a..next_a],
            &theirs_lines[b..next_b],
        );
        (i, a, b) = (j, next_a, next_b);
    }

    Ok(MergedNote {
        regions,
        trailing_newline: ours.is_empty() || ours.ends_with('\n'),
    })
}

/// Add a stretch both sides may have changed.
///
/// When all three have the same number of lines, line `k` of each is taken to
/// be the same line and settled on its own, so edits to neighbouring lines do
/// not clash and a conflict covers only the lines both sides changed.
fn push_changed(
    regions: &mut Vec<MergeRegion>,
    base: &[String],
    ours: &[String],
    theirs: &[String],
) {
    let aligned = !base.is_empty() && ours.len() == base.len() && theirs.len() == base.len();
    if !aligned {
        push_region(regions, classify(base, ours, theirs));
        return;
    }

    let kind = |k: usize| {
        let (b, o, t) = (&base[k], &ours[k], &theirs[k]);
        (o == b, t == b, o == t)
    };
    let mut start = 0;
    while start < base.len() {
        let mut end = start + 1;
        while end < base.len() && kind(end) == kind(start) {
            end += 1;
        }
        push_region(
            regions,
            classify(&base[start..end], &ours[start..end], &theirs[start..end]),
        );
        start = end;
    }
}

/// Append, joining runs of unchanged lines.
fn push_region(regions: &mut Vec<MergeRegion>, region: MergeRegion) {
    if let (Some(MergeRegion::Unchanged { lines: prev }), MergeRegion::Unchanged { lines }) =
        (regions.last_mut(), &region)
    {
        prev.extend(lines.iter().cloned());
        return;
    }
    regions.push(region);
}

fn classify(base: &[String], ours: &[String], theirs: &[String]) -> MergeRegion {
    let base_v = base.to_vec();
    if ours == base && theirs == base {
        MergeRegion::Unchanged { lines: base_v }
    } else if ours == base {
        MergeRegion::Theirs {
            base: base_v,
            lines: theirs.to_vec(),
        }
    } else if theirs == base {
        MergeRegion::Ours {
            base: base_v,
            lines: ours.to_vec(),
        }
    } else if ours == theirs {
        MergeRegion::Same {
            base: base_v,
            lines: ours.to_vec(),
        }
    } else {
        MergeRegion::Conflict {
            suggestion: suggest(base, ours, theirs),
            base: base_v,
            ours: ours.to_vec(),
            theirs: theirs.to_vec(),
        }
    }
}

fn suggest(base: &[String], ours: &[String], theirs: &[String]) -> Option<Suggestion> {
    if base.is_empty() {
        return Some(Suggestion {
            kind: SuggestionKind::Both,
            lines: ours.iter().chain(theirs).cloned().collect(),
        });
    }

    if ours.len() != base.len() || theirs.len() != base.len() {
        return None;
    }
    let lines = base
        .iter()
        .zip(ours)
        .zip(theirs)
        .map(|((b, o), t)| combine_line(b, o, t))
        .collect::<Option<Vec<_>>>()?;
    Some(Suggestion {
        kind: SuggestionKind::Combined,
        lines,
    })
}

/// A line as text segments around its `{@...}` properties:
/// `segments.len() == props.len() + 1`.
struct Parts<'a> {
    segments: Vec<&'a str>,
    props: Vec<&'a str>,
}

fn parts(line: &str) -> Parts<'_> {
    let mut segments = Vec::new();
    let mut props = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find("{@") {
        let Some(len) = rest[open..].find('}') else {
            break;
        };
        segments.push(&rest[..open]);
        props.push(&rest[open..open + len + 1]);
        rest = &rest[open + len + 1..];
    }
    segments.push(rest);
    Parts { segments, props }
}

/// Merge one line both sides changed, when one changed only its text and the
/// other only its properties.
fn combine_line(base: &str, ours: &str, theirs: &str) -> Option<String> {
    if ours == base || ours == theirs {
        return Some(theirs.to_string());
    }
    if theirs == base {
        return Some(ours.to_string());
    }

    let (b, o, t) = (parts(base), parts(ours), parts(theirs));
    // Adding or removing a property also moves the spaces around it, so text is
    // compared with whitespace collapsed.
    let text = |p: &Parts| {
        p.segments
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let props_only = |side: &Parts| text(side) == text(&b);
    let text_only = |side: &Parts| side.props == b.props;

    let (text_side, prop_side) = if props_only(&o) && text_only(&t) {
        (&t, &o)
    } else if props_only(&t) && text_only(&o) {
        (&o, &t)
    } else {
        return None;
    };

    // Same number of properties: put the new ones where the old ones were.
    if text_side.props.len() == prop_side.props.len() {
        let mut line = String::new();
        for (i, segment) in text_side.segments.iter().enumerate() {
            line.push_str(segment);
            if let Some(prop) = prop_side.props.get(i) {
                line.push_str(prop);
            }
        }
        return Some(line);
    }

    // Otherwise only the common shape is safe: properties trailing the text.
    let trailing = |p: &Parts| p.segments[1..].iter().all(|s| s.trim().is_empty());
    if !trailing(text_side) || !trailing(prop_side) {
        return None;
    }
    let text = text_side.segments[0].trim_end();
    let props = prop_side.props.join(" ");
    Some(if props.is_empty() {
        text.to_string()
    } else if text.trim().is_empty() {
        format!("{text}{props}")
    } else {
        format!("{text} {props}")
    })
}

/// The text a set of regions stands for once each conflict has been given its
/// lines. `choices` holds one entry per conflict, in order.
pub fn assemble(note: &MergedNote, choices: &[Vec<String>]) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut choice = choices.iter();
    for region in &note.regions {
        let picked: &[String] = match region {
            MergeRegion::Unchanged { lines }
            | MergeRegion::Ours { lines, .. }
            | MergeRegion::Theirs { lines, .. }
            | MergeRegion::Same { lines, .. } => lines,
            MergeRegion::Conflict { .. } => choice.next().map(Vec::as_slice).unwrap_or(&[]),
        };
        lines.extend(picked.iter().map(String::as_str));
    }
    let mut text = lines.join("\n");
    if note.trailing_newline && !lines.is_empty() {
        text.push('\n');
    }
    text
}
