use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use clap::{Parser as ClapParser, Subcommand};
use serde::Serialize;
use url::Url;

use patto::ast_query::task_label;
use patto::parser::{Deadline, TaskStatus};
use patto::repository::{DateRange, Repository, TaskQuery};
use patto::task_edits::set_task_status_in_text;
use patto::tasks_view::{completed_group, pending_group, timeframe_bounds, ReviewTimeframe};

#[derive(ClapParser)]
#[command(
    version,
    about = "Query a patto notes directory and print JSON",
    long_about = "One-shot queries for tools that cannot hold an LSP session, \
                  such as editor and terminal plugins. Every command prints JSON on stdout."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open tasks grouped by deadline, plus done tasks in a timeframe
    Tasks {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Done tasks to include: none, today, yesterday, this_week, last_week, this_month
        #[arg(long, default_value = "none")]
        done: String,
        /// Override today's date (YYYY-MM-DD)
        #[arg(long)]
        today: Option<NaiveDate>,
    },
    /// Back links and two-hop links of one note
    Links {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        file: PathBuf,
    },
    /// Set the status of the task on a line, recording timestamps like the LSP
    SetStatus {
        #[arg(long)]
        file: PathBuf,
        /// 1-based line number
        #[arg(long)]
        line: usize,
        /// todo, doing, paused, done or archived
        status: String,
    },
    /// Append a line, or a task, to a note
    Capture {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        task: bool,
        /// Deadline for a task (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
        #[arg(long)]
        due: Option<String>,
        text: String,
    },
}

#[derive(Serialize)]
struct TaskItem {
    file: PathBuf,
    line: usize,
    text: String,
    status: &'static str,
    due: Option<String>,
    group: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_on: Option<NaiveDate>,
}

#[derive(Serialize)]
struct TasksOutput {
    root: PathBuf,
    today: NaiveDate,
    open: Vec<TaskItem>,
    done: Vec<TaskItem>,
}

#[derive(Serialize)]
struct NoteRef {
    name: String,
    file: Option<PathBuf>,
}

#[derive(Serialize)]
struct BackLinkLine {
    line: usize,
    context: String,
}

#[derive(Serialize)]
struct BackLink {
    #[serde(flatten)]
    note: NoteRef,
    lines: Vec<BackLinkLine>,
}

#[derive(Serialize)]
struct TwoHop {
    bridge: NoteRef,
    notes: Vec<NoteRef>,
}

#[derive(Serialize)]
struct LinksOutput {
    file: PathBuf,
    backlinks: Vec<BackLink>,
    two_hop: Vec<TwoHop>,
}

#[derive(Serialize)]
struct LineOutput {
    file: PathBuf,
    line: usize,
    text: String,
}

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Tasks { root, done, today } => {
            let today = today.unwrap_or_else(|| Local::now().date_naive());
            tasks(&root, &done, today).and_then(print)
        }
        Command::Links { root, file } => links(&root, &file).and_then(print),
        Command::SetStatus { file, line, status } => {
            set_status(&file, line, &status).and_then(print)
        }
        Command::Capture {
            file,
            task,
            due,
            text,
        } => capture(&file, &text, task, due.as_deref()).and_then(print),
    };
    if let Err(error) = result {
        eprintln!("patto-query: {error}");
        std::process::exit(1);
    }
}

fn print<T: Serialize>(value: T) -> AnyResult<()> {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, &value)?;
    writeln!(out)?;
    Ok(())
}

fn load(root: &Path) -> AnyResult<Repository> {
    let root = fs::canonicalize(root)?;
    let repository = Repository::new(root);
    repository.load_all();
    Ok(repository)
}

fn done_range(name: &str, today: NaiveDate) -> Option<DateRange> {
    if name == "none" {
        return None;
    }
    let (from, to) = timeframe_bounds(&ReviewTimeframe::from_name(name, None, None), today);
    Some(DateRange { from, to })
}

fn deadline_text(due: &Deadline) -> Option<String> {
    let text = due.to_string();
    (!text.is_empty()).then_some(text)
}

fn tasks(root: &Path, done: &str, today: NaiveDate) -> AnyResult<TasksOutput> {
    let repository = load(root)?;
    let query = TaskQuery {
        open: true,
        done: done_range(done, today),
        archived: None,
    };
    let mut output = TasksOutput {
        root: repository.root_dir.clone(),
        today,
        open: Vec::new(),
        done: Vec::new(),
    };
    for (uri, task) in repository.tasks(&query) {
        let Ok(file) = uri.to_file_path() else {
            continue;
        };
        let due = deadline_text(&task.due);
        let item = |group, completed_on| TaskItem {
            file,
            line: task.line.location().row + 1,
            text: task_label(&task.line),
            status: task.status.keyword(),
            due,
            group,
            completed_on,
        };
        match (&task.status, task.resolved_on) {
            (TaskStatus::Done, Some(date)) => output
                .done
                .push(item(completed_group(date, today).label(), Some(date))),
            (TaskStatus::Done | TaskStatus::Archived, _) => {}
            (_, _) if matches!(&task.due, Deadline::Uninterpretable(s) if s.is_empty()) => {
                output.open.push(item("No Deadline", None))
            }
            _ => output
                .open
                .push(item(pending_group(&task.due, today).label(), None)),
        }
    }
    Ok(output)
}

fn note_ref(repository: &Repository, root_uri: &Url, name: String) -> NoteRef {
    let file = repository
        .link_to_uri(&name, root_uri)
        .and_then(|uri| uri.to_file_path().ok())
        .filter(|path| path.exists());
    NoteRef { name, file }
}

fn line_text(repository: &Repository, file: Option<&Path>, row: usize) -> String {
    file.and_then(|path| Url::from_file_path(path).ok())
        .and_then(|uri| {
            let rope = repository.document_map.get(&uri)?;
            let line = rope.get_line(row)?;
            Some(line.to_string().trim().to_string())
        })
        .unwrap_or_default()
}

fn links(root: &Path, file: &Path) -> AnyResult<LinksOutput> {
    let repository = load(root)?;
    let file = fs::canonicalize(file)?;
    let root_uri = Url::from_directory_path(&repository.root_dir)
        .map_err(|_| "notes directory is not an absolute path")?;

    let backlinks = repository
        .calculate_back_links(&file)
        .into_iter()
        .map(|back_link| {
            let note = note_ref(&repository, &root_uri, back_link.source_file);
            let lines = back_link
                .locations
                .iter()
                .map(|location| BackLinkLine {
                    line: location.line + 1,
                    context: line_text(&repository, note.file.as_deref(), location.line),
                })
                .collect();
            BackLink { note, lines }
        })
        .collect();

    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    let two_hop = runtime
        .block_on(repository.calculate_two_hop_links(&file))
        .into_iter()
        .map(|(bridge, notes)| TwoHop {
            bridge: note_ref(&repository, &root_uri, bridge),
            notes: notes
                .into_iter()
                .map(|name| note_ref(&repository, &root_uri, name))
                .collect(),
        })
        .collect();

    Ok(LinksOutput {
        file,
        backlinks,
        two_hop,
    })
}

fn set_status(file: &Path, line: usize, status: &str) -> AnyResult<LineOutput> {
    let status =
        TaskStatus::from_keyword(status).ok_or_else(|| format!("unknown status: {status}"))?;
    let row = line.checked_sub(1).ok_or("lines start at 1")?;
    let text = fs::read_to_string(file)?;
    let updated = set_task_status_in_text(&text, row, status, Local::now().naive_local())
        .ok_or_else(|| format!("no task on line {line}"))?;
    fs::write(file, &updated)?;
    Ok(LineOutput {
        file: fs::canonicalize(file)?,
        line,
        text: updated.split('\n').nth(row).unwrap_or_default().to_string(),
    })
}

fn capture(file: &Path, text: &str, task: bool, due: Option<&str>) -> AnyResult<LineOutput> {
    let text = text.trim();
    if text.is_empty() || text.contains('\n') {
        return Err("capture takes one non-empty line".into());
    }
    let line_text = match (task, due) {
        (true, Some(due)) => format!("{text} {{@task status=todo due={due}}}"),
        (true, None) => format!("{text} {{@task status=todo}}"),
        (false, _) => text.to_string(),
    };

    if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut content = fs::read_to_string(file).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&line_text);
    content.push('\n');
    fs::write(file, &content)?;

    Ok(LineOutput {
        file: fs::canonicalize(file)?,
        line: content.lines().count(),
        text: line_text,
    })
}
