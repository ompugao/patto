//! The inbox note: quick posts grouped under date headings.
//!
//! ```text
//! 2026-10-07
//! 	09:12 bought coffee beans
//! 	11:40 idea for the parser
//! 		continuation line
//! ```
//!
//! Lines that do not fit (hand-edited content) are left alone when appending
//! and skipped when listing.

use std::path::Path;

use crate::api::error::{PattoError, PattoResult};
use crate::api::store::{meta_for, name_to_rel_path, resolve, write_note};
use crate::api::types::{InboxPost, NoteMeta};

fn is_date_heading(line: &str) -> bool {
    let b = line.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

/// `HH:mm ` followed by the post's first line.
fn split_post(line: &str) -> Option<(&str, &str)> {
    let b = line.as_bytes();
    let well_formed = b.len() > 6
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2] == b':'
        && b[3].is_ascii_digit()
        && b[4].is_ascii_digit()
        && b[5] == b' ';
    well_formed.then(|| (&line[..5], &line[6..]))
}

fn depth_of(line: &str) -> usize {
    line.bytes().take_while(|c| *c == b'\t').count()
}

pub fn parse_posts(content: &str) -> Vec<InboxPost> {
    let mut posts: Vec<InboxPost> = Vec::new();
    let mut date: Option<String> = None;
    let mut in_post = false;

    for (index, raw) in content.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        let depth = depth_of(line);
        let text = &line[depth..];
        if text.trim().is_empty() {
            continue;
        }
        match depth {
            0 => {
                date = is_date_heading(text).then(|| text.to_string());
                in_post = false;
            }
            1 => {
                in_post = false;
                if let (Some(date), Some((time, first))) = (&date, split_post(text)) {
                    posts.push(InboxPost {
                        date: date.clone(),
                        time: time.to_string(),
                        text: first.to_string(),
                        body: Vec::new(),
                        line: index as u32,
                    });
                    in_post = true;
                }
            }
            _ => {
                if in_post {
                    if let Some(post) = posts.last_mut() {
                        post.body.push(line[2..].to_string());
                    }
                }
            }
        }
    }
    posts
}

fn read_or_empty(path: &Path) -> PattoResult<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(PattoError::Io(e.to_string())),
    }
}

/// Every post in the inbox note, oldest first. A missing note has no posts.
pub fn inbox_posts(root: String, name: String) -> PattoResult<Vec<InboxPost>> {
    let rel_path = name_to_rel_path(&name)?;
    let content = read_or_empty(&resolve(&root, &rel_path)?)?;
    Ok(parse_posts(&content))
}

/// The post as it is written: `HH:mm first line`, further lines nested one
/// level deeper. Blank lines are dropped. Empty when there is nothing to say.
pub fn format_post(time: &str, text: &str) -> String {
    let mut lines = text
        .replace("\r\n", "\n")
        .split('\n')
        .map(|l| l.trim_end())
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>()
        .into_iter();
    let Some(first) = lines.next() else {
        return String::new();
    };
    let mut out = format!("\t{} {}", time, first.trim_start_matches('\t'));
    for line in lines {
        out.push_str("\n\t\t");
        out.push_str(&line);
    }
    out
}

/// The last top-level line, if it is a date heading.
fn last_heading(content: &str) -> Option<&str> {
    content
        .lines()
        .rev()
        .find(|l| depth_of(l) == 0 && !l.trim().is_empty())
        .filter(|l| is_date_heading(l.trim_end_matches('\r')))
}

/// Append a post under `date`, adding the heading when the note does not end
/// with it, and create the note if it does not exist yet.
pub fn inbox_append(
    root: String,
    name: String,
    date: String,
    time: String,
    text: String,
) -> PattoResult<NoteMeta> {
    let post = format_post(&time, &text);
    if post.is_empty() {
        return Err(PattoError::Io("nothing to post".to_string()));
    }
    let rel_path = name_to_rel_path(&name)?;
    let path = resolve(&root, &rel_path)?;
    let mut content = read_or_empty(&path)?.replace("\r\n", "\n");

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if last_heading(&content) != Some(date.as_str()) {
        content.push_str(&date);
        content.push('\n');
    }
    content.push_str(&post);
    content.push('\n');

    write_note(root.clone(), rel_path.clone(), content)?;
    meta_for(Path::new(&root), &path).ok_or(PattoError::NotFound(rel_path))
}
