// Patto Mobile - Tauri 2.0 Application
// A mobile app for viewing and editing patto notes

mod commands;

use commands::files::{create_file, delete_file, get_file_info, list_files, rename_file};
use commands::git::{configure_remote, git_clone, git_init, git_pull, git_status, git_sync};
use commands::notes::{extract_links, read_note, render_content, render_note, write_note};
use commands::tasks::{get_all_tasks, get_file_tasks, get_task_summary};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Git commands
            git_clone,
            git_pull,
            git_sync,
            git_init,
            git_status,
            configure_remote,
            // File commands
            list_files,
            get_file_info,
            create_file,
            delete_file,
            rename_file,
            // Note commands
            read_note,
            write_note,
            render_note,
            render_content,
            extract_links,
            // Task commands
            get_all_tasks,
            get_file_tasks,
            get_task_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
