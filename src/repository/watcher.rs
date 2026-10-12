use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::time::sleep;

use super::{FileMetadata, Repository, RepositoryMessage, WORKSPACE_CONFIG_FILENAME};

impl Repository {
    /// Start watching the repository directory for `.pn` and workspace-config changes.
    pub async fn start_watcher(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        // Registered before returning, so changes made right after this call are seen.
        let watcher = watch_dir(&self.root_dir, tx)?;
        self.clone().spawn_event_loop(rx, watcher);
        Ok(())
    }

    fn spawn_event_loop(self, mut rx: mpsc::Receiver<notify::Event>, watcher: RecommendedWatcher) {
        let debouncer = Debouncer::new(Duration::from_millis(10));
        tokio::spawn(async move {
            // A watcher stops delivering events once dropped, so the loop that
            // consumes them owns it, and both end together.
            let _watcher = watcher;
            while let Some(event) = rx.recv().await {
                self.handle_fs_event(event, &debouncer).await;
            }
        });
    }

    async fn handle_fs_event(&self, event: notify::Event, debouncer: &Arc<Debouncer>) {
        if !(event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove()) {
            return;
        }

        for path in event.paths {
            if path.file_name().and_then(|n| n.to_str()) == Some(WORKSPACE_CONFIG_FILENAME) {
                if event.kind.is_modify() || event.kind.is_create() {
                    self.reload_workspace_config();
                }
                continue;
            }

            if path.extension().and_then(|s| s.to_str()) != Some("pn") {
                continue;
            }

            if event.kind.is_create() {
                self.handle_file_created(&path);
            } else if event.kind.is_remove() {
                self.handle_file_removed(&path);
            } else if event.kind.is_modify() && path.is_file() {
                self.spawn_debounced_reload(path, debouncer.clone());
            }
        }
    }

    fn handle_file_created(&self, path: &Path) {
        if !path.starts_with(&self.root_dir) {
            return;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        self.add_file_to_graph(path, &content);

        let Ok(metadata) = self.collect_file_metadata(path) else {
            return;
        };
        let _ = self
            .tx
            .send(RepositoryMessage::FileAdded(path.to_path_buf(), metadata));
    }

    fn handle_file_removed(&self, path: &Path) {
        if !path.starts_with(&self.root_dir) {
            return;
        }
        self.remove_file_from_graph(path);
        let _ = self
            .tx
            .send(RepositoryMessage::FileRemoved(path.to_path_buf()));
    }

    /// Editors write a file in several bursts, so wait out the burst and reload
    /// only once the path has been quiet for the debounce window.
    fn spawn_debounced_reload(&self, path: PathBuf, debouncer: Arc<Debouncer>) {
        debouncer.record(&path);

        let repository = self.clone();
        tokio::spawn(async move {
            sleep(debouncer.window).await;
            if !debouncer.take_if_settled(&path) {
                return;
            }
            let Ok(content) = tokio::fs::read_to_string(&path).await else {
                return;
            };
            repository.handle_live_file_change(path, content).await;
        });
    }

    /// Broadcast a file change with provided content without relying on the filesystem watcher
    pub async fn handle_live_file_change(&self, path: PathBuf, content: String) {
        if !path.starts_with(&self.root_dir) {
            return;
        }

        self.add_file_to_graph(&path, &content);

        let back_links = self.calculate_back_links(&path);
        let link_count = back_links
            .iter()
            .map(|back_link| back_link.locations.len() as u32)
            .sum::<u32>();

        let metadata = self.collect_file_metadata(&path).unwrap_or_else(|_| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            FileMetadata {
                modified: now,
                created: now,
                link_count,
            }
        });

        let two_hop_links = self.calculate_two_hop_links(&path).await;

        let _ = self.tx.send(RepositoryMessage::FileChanged(
            path.clone(),
            metadata,
            content,
        ));
        let _ = self.tx.send(RepositoryMessage::BackLinksChanged(
            path.clone(),
            back_links,
        ));
        let _ = self
            .tx
            .send(RepositoryMessage::TwoHopLinksChanged(path, two_hop_links));
    }
}

/// Register a recursive watch on `dir`, forwarding its events to `tx`.
fn watch_dir(dir: &Path, tx: mpsc::Sender<notify::Event>) -> notify::Result<RecommendedWatcher> {
    let mut watcher = RecommendedWatcher::new(
        // `notify` invokes this on its own thread, never on a runtime worker.
        move |result| {
            if let Ok(event) = result {
                let _ = tx.blocking_send(event);
            }
        },
        Config::default(),
    )?;
    watcher.watch(dir, RecursiveMode::Recursive)?;
    Ok(watcher)
}

/// Collapses a burst of modify events into a single reload per path.
struct Debouncer {
    pending: Mutex<HashMap<PathBuf, Instant>>,
    window: Duration,
}

impl Debouncer {
    fn new(window: Duration) -> Arc<Self> {
        Arc::new(Self {
            pending: Mutex::new(HashMap::new()),
            window,
        })
    }

    fn record(&self, path: &Path) {
        self.pending
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), Instant::now());
    }

    /// `true` if `path` has been quiet for the whole window, in which case the
    /// entry is consumed so only one waiter reloads it.
    fn take_if_settled(&self, path: &Path) -> bool {
        let mut pending = self.pending.lock().unwrap();
        let Some(&last_change) = pending.get(path) else {
            return false;
        };
        if Instant::now().duration_since(last_change) < self.window {
            return false;
        }
        pending.remove(path);
        true
    }
}
