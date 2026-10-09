//! The `patto-markdown-importer` command line, as scripts drive it.
#![cfg(feature = "cli")]

use std::path::Path;
use std::process::{Command, Output};

fn importer() -> Command {
    Command::new(env!("CARGO_BIN_EXE_patto-markdown-importer"))
}

fn run(command: &mut Command) -> Output {
    command
        .output()
        .expect("failed to run patto-markdown-importer")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn write(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[test]
fn a_single_file_converts_to_stdout_by_default() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "note.md", "Hello **there**\n");

    let output = run(importer().arg("-f").arg(dir.path().join("note.md")));

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello [* there]\n");
}

#[test]
fn stdin_is_read_when_no_file_is_given() {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = importer()
        .arg("--mode")
        .arg("lossy")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"plain text\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "plain text\n");
}

#[test]
fn strict_mode_fails_on_unsupported_markdown() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "note.md", "<div>html</div>\n");

    let output = run(importer()
        .arg("-f")
        .arg(dir.path().join("note.md"))
        .arg("--mode")
        .arg("strict"));

    assert!(!output.status.success());
    assert!(stderr(&output).contains("HTML is not supported"));
}

#[test]
fn an_output_file_is_written_with_a_report() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "note.md", "# Title\n\ntext\n");
    let out = dir.path().join("note.pn");
    let report = dir.path().join("report.json");

    let output = run(importer()
        .arg("-f")
        .arg(dir.path().join("note.md"))
        .arg("-o")
        .arg(&out)
        .arg("--mode")
        .arg("lossy")
        .arg("--report")
        .arg(&report));

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "Title\n-----\ntext\n"
    );
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(report["mode"], "Lossy");
    assert_eq!(report["warnings"].as_array().unwrap().len(), 1);
    let messages = stderr(&output);
    assert!(messages.contains("Converted"), "{messages}");
    assert!(messages.contains("Report written"), "{messages}");
}

#[test]
fn a_dry_run_prints_the_report_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "note.md", "text\n");
    let out = dir.path().join("note.pn");

    let output = run(importer()
        .arg("-f")
        .arg(dir.path().join("note.md"))
        .arg("-o")
        .arg(&out)
        .arg("--dry-run"));

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!out.exists());
    assert!(
        stderr(&output).contains("Markdown Import Report"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn batch_conversion_requires_an_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(importer().arg("-d").arg(dir.path()));
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Output directory required"));
}

#[test]
fn batch_conversion_mirrors_the_directory_tree() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    write(&input, "a.md", "a\n");
    write(&input, "sub/b.md", "b\n");
    write(&input, "notes.txt", "ignored\n");

    let output = run(importer()
        .arg("-d")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--mode")
        .arg("lossy"));

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(std::fs::read_to_string(out.join("a.pn")).unwrap(), "a\n");
    assert!(
        !out.join("sub").join("b.pn").exists(),
        "default pattern is flat"
    );
    let messages = stderr(&output);
    assert!(messages.contains("Files processed: 1"), "{messages}");
    assert!(messages.contains("Succeeded:       1"), "{messages}");
}

#[test]
fn batch_conversion_follows_a_recursive_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    write(&input, "a.md", "a\n");
    write(&input, "sub/b.md", "b\n");

    let output = run(importer()
        .arg("-d")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--pattern")
        .arg("**/*.md")
        .arg("--mode")
        .arg("lossy"));

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(std::fs::read_to_string(out.join("a.pn")).unwrap(), "a\n");
    assert_eq!(
        std::fs::read_to_string(out.join("sub").join("b.pn")).unwrap(),
        "b\n"
    );
}

#[test]
fn batch_report_counts_the_files_that_failed() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    let report = dir.path().join("batch.json");
    write(&input, "good.md", "# ok\n");
    write(&input, "bad.md", "<div>x</div>\n");

    let output = run(importer()
        .arg("-d")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--mode")
        .arg("strict")
        .arg("--report")
        .arg(&report));

    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(report["files_processed"], 2);
    assert_eq!(report["files_succeeded"], 1);
    assert_eq!(report["files_failed"], 1);
    assert_eq!(report["total_warnings"], 1);
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(report["files"][0]["status"], "success_with_warnings");
    let messages = stderr(&output);
    assert!(messages.contains("Failed:          1"), "{messages}");
}

#[test]
fn batch_text_report_lists_each_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    let report = dir.path().join("batch.txt");
    write(&input, "a.md", "a\n");

    let output = run(importer()
        .arg("-d")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--report")
        .arg(&report)
        .arg("--report-format")
        .arg("text"));

    assert!(output.status.success(), "{}", stderr(&output));
    let text = std::fs::read_to_string(&report).unwrap();
    assert!(text.starts_with("Batch Conversion Report\n"), "{text}");
    assert!(text.contains("Files processed:  1\n"), "{text}");
    assert!(text.contains("✓ "), "{text}");
    assert!(text.contains("a.md -> "), "{text}");
}

#[test]
fn a_batch_dry_run_writes_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    write(&input, "a.md", "a\n");

    let output = run(importer()
        .arg("-d")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--dry-run"));

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!out.join("a.pn").exists());
    assert!(stderr(&output).contains("Dry run"), "{}", stderr(&output));
}
