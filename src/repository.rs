use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use gdsl::sync_digraph::Graph;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio::time::sleep;
use tower_lsp::lsp_types::Url;
use urlencoding::encode;

use crate::parser::{self, AstNode, Location};

/// Location information for a WikiLink
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkLocation {
    /// Source line number (0-indexed)
    pub source_line: usize,
    /// Source column range (byte offsets within the line)
    pub source_col_range: (usize, usize),
    /// Target anchor name (if linking to specific anchor)
    pub target_anchor: Option<String>,
}

/// Edge data for document graph connections
#[derive(Debug, Clone)]
pub struct LinkEdge {
    /// All link locations from source to target document
    pub locations: Vec<LinkLocation>,
}

/// Link location data for preview (serializable)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinkLocationData {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column range within the line
    pub col_range: (usize, usize),
    /// Optional: text context around the link
    pub context: Option<String>,
    /// Target anchor (if linking to specific anchor)
    pub target_anchor: Option<String>,
}

/// Backlink data with locations for preview
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackLinkData {
    /// Source file name (link name, not full path)
    pub source_file: String,
    /// All locations in source file that link here
    pub locations: Vec<LinkLocationData>,
}


/// File metadata for sorting and display
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileMetadata {
    pub modified: u64, // Unix timestamp
    pub created: u64,  // Unix timestamp
    #[serde(rename = "linkCount")]
    pub link_count: u32,
}

/// Messages for repository change notifications
#[derive(Clone, Debug)]
pub enum RepositoryMessage {
    FileChanged(PathBuf, FileMetadata, String),
    FileAdded(PathBuf, FileMetadata),
    FileRemoved(PathBuf),
    BackLinksChanged(PathBuf, Vec<BackLinkData>),
    TwoHopLinksChanged(PathBuf, Vec<(String, Vec<String>)>),
    ScanStarted { total_files: usize },
    ScanProgress { scanned: usize, total: usize },
    ScanCompleted { total_files: usize },
}

/// Repository manages the collection of notes and their relationships
#[derive(Clone)]
pub struct Repository {
    /// Root directory of the repository
    pub root_dir: PathBuf,

    /// Broadcast channel for change notifications
    pub tx: broadcast::Sender<RepositoryMessage>,

    /// Graph structure for note relationships (used by LSP)
    pub document_graph: Arc<Mutex<Graph<Url, AstNode, LinkEdge>>>,

    /// Simple link graph for web preview (PathBuf -> linked PathBufs)
    pub link_graph: Arc<Mutex<HashMap<PathBuf, HashSet<PathBuf>>>>,

    /// AST cache for parsed documents
    pub ast_map: Arc<DashMap<Url, AstNode>>,

    /// Document content cache
    pub document_map: Arc<DashMap<Url, ropey::Rope>>,
}

impl Repository {
    /// Create a new repository and build initial document graph
    pub fn new(root_dir: PathBuf) -> Self {
        let (tx, _) = broadcast::channel(100);

        let repo = Self {
            root_dir,
            tx,
            document_graph: Arc::new(Mutex::new(Graph::new())),
            link_graph: Arc::new(Mutex::new(HashMap::new())),
            ast_map: Arc::new(DashMap::new()),
            document_map: Arc::new(DashMap::new()),
        };

        // Spawn background task for initial scanning to avoid blocking
        let repo_clone = repo.clone();
        tokio::spawn(async move {
            repo_clone.build_initial_graph().await;
        });

        repo
    }

    /// Subscribe to repository change notifications
    pub fn subscribe(&self) -> broadcast::Receiver<RepositoryMessage> {
        self.tx.subscribe()
    }

    /// Gather wikilinks with their source locations
    pub fn gather_wikilinks(
        parent: &AstNode,
        wikilinks: &mut Vec<(String, Option<String>, Location)>,
    ) {
        if let parser::AstNodeKind::WikiLink { link, anchor } = &parent.kind() {
            wikilinks.push((link.clone(), anchor.clone(), parent.location().clone()));
        }

        for content in parent.value().contents.lock().unwrap().iter() {
            Self::gather_wikilinks(content, wikilinks);
        }

        for child in parent.value().children.lock().unwrap().iter() {
            Self::gather_wikilinks(child, wikilinks);
        }
    }

    /// Convert link name to file path
    pub fn link_to_path(&self, link: &str) -> Option<PathBuf> {
        if !link.is_empty() {
            let file_path = self.root_dir.join(format!("{}.pn", link));
            if file_path.exists() {
                Some(file_path)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Convert file path to link name
    pub fn path_to_link(&self, path: &Path) -> Option<String> {
        if let Ok(rel_path) = path.strip_prefix(&self.root_dir) {
            if let Some(stem) = rel_path.file_stem() {
                if let Some(stem_str) = stem.to_str() {
                    return Some(stem_str.to_string());
                }
            }
        }
        None
    }

    /// Convert link to URI (for LSP integration)
    pub fn link_to_uri(&self, link: &str, root_uri: &Url) -> Option<Url> {
        if !link.is_empty() {
            fn ensure_trailing_slash(s: &str) -> String {
                if s.ends_with('/') {
                    s.to_string()
                } else {
                    format!("{}/", s)
                }
            }
            let mut linkuri = root_uri.clone();
            linkuri.set_path(
                format!(
                    "{}{}.pn",
                    ensure_trailing_slash(root_uri.path()),
                    encode(link)
                )
                .as_str(),
            );
            Some(Self::normalize_url_percent_encoding(&linkuri))
        } else {
            None
        }
    }

    /// Normalize URL percent encoding
    pub fn normalize_url_percent_encoding(url: &Url) -> Url {
        let re = regex::Regex::new(r"%[0-9a-fA-F]{2}").unwrap();
        let normalized = re.replace_all(url.as_str(), |caps: &regex::Captures| {
            caps[0].to_uppercase()
        });

        Url::parse(&normalized).unwrap_or(url.clone())
    }

    //// Count links in a patto file using the parser
    // pub fn count_links_in_file(&self, path: &Path) -> std::io::Result<u32> {
    //     let content = std::fs::read_to_string(path)?;
    //     let result = parser::parse_text(&content);
    //     let mut wikilinks = vec![];
    //     Self::gather_wikilinks(&result.ast, &mut wikilinks);
    //     Ok(wikilinks.len() as u32)
    // }

    /// Collect patto files with metadata
    pub fn collect_patto_files_with_metadata(
        &self,
        dir: &Path,
        files: &mut Vec<String>,
        metadata: &mut HashMap<String, FileMetadata>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    self.collect_patto_files_with_metadata(&path, files, metadata);
                } else if path.extension().and_then(|s| s.to_str()) == Some("pn") {
                    if let Ok(rel_path) = path.strip_prefix(&self.root_dir) {
                        let rel_path_str = rel_path.to_string_lossy().to_string();
                        files.push(rel_path_str.clone());

                        metadata.insert(rel_path_str, self.collect_file_metadata(&path).unwrap());
                    }
                }
            }
        }
    }

    pub fn collect_file_metadata(&self, file_path: &PathBuf) -> std::io::Result<FileMetadata> {
        let file_metadata = std::fs::metadata(file_path)?;
        let modified = file_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let created = file_metadata
            .created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let link_count = self.count_back_links(file_path);

        Ok(FileMetadata {
            modified,
            created,
            link_count: link_count.try_into().unwrap(),
        })
    }

    /// Update only the specific file's links in the graph (deprecated - use add_file_to_graph)
    pub fn update_links_in_graph(&self, file_path: &Path, content: &str) {
        // This method is now handled by add_file_to_graph which updates the document graph
        self.add_file_to_graph(file_path, content);
    }

    /// Extract context around a link location
    fn extract_context(
        rope: &ropey::Rope,
        line: usize,
        col_range: (usize, usize),
    ) -> Option<String> {
        if line >= rope.len_lines() {
            return None;
        }

        let line_start = rope.line_to_char(line);
        let line_end = if line + 10 < rope.len_lines() {
            rope.line_to_char(line + 10)
        } else {
            rope.len_chars()
        };

        let line_slice = rope.slice(line_start..line_end);
        let line_str = line_slice.to_string();

        // Get surrounding context (e.g., full line or trimmed)
        const MAX_CONTEXT_LEN: usize = 80;

        // let start = col_range.0.saturating_sub(20).max(0);
        // let end = (col_range.1 + 20).min(line_str.len());

        let mut context = line_str;

        // Truncate if too long
        if context.len() > MAX_CONTEXT_LEN {
            context.truncate(MAX_CONTEXT_LEN - 3);
            context.push_str("...");
        }

        Some(context)
    }

    /// Calculate back-links for a specific file using document graph
    pub fn calculate_back_links(&self, file_path: &Path) -> Vec<BackLinkData> {
        // Security check
        let canonical_base = match std::fs::canonicalize(&self.root_dir) {
            Ok(base) => base,
            Err(_) => return Vec::new(),
        };

        let canonical_file = match std::fs::canonicalize(file_path) {
            Ok(file) => file,
            Err(_) => return Vec::new(),
        };

        if !canonical_file.starts_with(&canonical_base) {
            return Vec::new();
        }

        // Get URI for the file
        let Ok(uri) = Url::from_file_path(&canonical_file) else {
            return Vec::new();
        };
        let uri = Self::normalize_url_percent_encoding(&uri);

        // Get back-links from document graph with locations
        let mut result = Vec::new();

        if let Ok(graph) = self.document_graph.lock() {
            if let Some(node) = graph.get(&uri) {
                for edge in node.iter_in() {
                    let source_uri = edge.source().key();
                    let edge_data = edge.value();

                    if let Ok(source_path) = source_uri.to_file_path() {
                        if let Some(source_file) = self.path_to_link(&source_path) {
                            // Get content for context extraction
                            let context_rope = self.document_map.get(source_uri);

                            let locations: Vec<LinkLocationData> = edge_data
                                .locations
                                .iter()
                                .map(|loc| {
                                    // Extract text context
                                    let context = context_rope.as_ref().and_then(|rope| {
                                        Self::extract_context(
                                            rope.value(),
                                            loc.source_line,
                                            loc.source_col_range,
                                        )
                                    });

                                    LinkLocationData {
                                        line: loc.source_line,
                                        col_range: loc.source_col_range,
                                        context,
                                        target_anchor: loc.target_anchor.clone(),
                                    }
                                })
                                .collect();

                            result.push(BackLinkData {
                                source_file,
                                locations,
                            });
                        }
                    }
                }
            }
        }

        // Sort by file name
        result.sort_by(|a, b| a.source_file.cmp(&b.source_file));
        result
    }

    /// Helper method to get backlink count (for metadata)
    pub fn count_back_links(&self, file_path: &Path) -> usize {
        let back_links = self.calculate_back_links(file_path);
        back_links.iter().map(|bl| bl.locations.len()).sum()
    }

    /// Calculate two-hop links for a specific file using document graph
    pub async fn calculate_two_hop_links(&self, file_path: &Path) -> Vec<(String, Vec<String>)> {
        // Security check
        let canonical_base = match std::fs::canonicalize(&self.root_dir) {
            Ok(base) => base,
            Err(_) => return Vec::new(),
        };

        let canonical_file = match std::fs::canonicalize(file_path) {
            Ok(file) => file,
            Err(_) => return Vec::new(),
        };

        if !canonical_file.starts_with(&canonical_base) {
            return Vec::new();
        }

        // Get URI for the file
        let Ok(uri) = Url::from_file_path(&canonical_file) else {
            return Vec::new();
        };
        let uri = Self::normalize_url_percent_encoding(&uri);

        let mut two_hop_links: Vec<(String, Vec<String>)> = Vec::new();

        if let Ok(graph) = self.document_graph.lock() {
            if let Some(node) = graph.get(&uri) {
                // For each file this file links to (direct links)
                for edge in node.iter_out() {
                    let target_uri = edge.target().key();
                    let target_node = edge.target();

                    // Find other files that also link to this same target (two-hop connections)
                    let mut connected_files = Vec::new();

                    for incoming_edge in target_node.iter_in() {
                        let source_uri = incoming_edge.source().key();
                        // Skip self and the direct target
                        if source_uri != &uri && source_uri != target_uri {
                            if let Ok(source_path) = source_uri.to_file_path() {
                                if let Some(link_name) = self.path_to_link(&source_path) {
                                    connected_files.push(link_name);
                                }
                            }
                        }
                    }

                    if !connected_files.is_empty() {
                        // Get the bridge link name (the file that connects us to others)
                        if let Ok(target_path) = target_uri.to_file_path() {
                            if let Some(bridge_link_name) = self.path_to_link(&target_path) {
                                connected_files.sort();
                                two_hop_links.push((bridge_link_name, connected_files));
                            }
                        }
                    }
                }
            }
        }

        // Sort by number of connections (descending)
        two_hop_links.sort_by_key(|(_, connections)| -(connections.len() as i32));
        two_hop_links
    }

    /// Build initial document graph by scanning all files
    async fn build_initial_graph(&self) {
        // Collect all files first to know total count
        let files = self.collect_pn_files(&self.root_dir);
        let total = files.len();
        
        // Send start message
        let _ = self.tx.send(RepositoryMessage::ScanStarted { 
            total_files: total 
        });
        
        // Process files with progress updates
        for (idx, file_path) in files.iter().enumerate() {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                self.add_file_to_graph(file_path, &content);
            }
            
            tokio::task::yield_now().await;
            // Report progress every 10 files or on last file
            if (idx + 1) % 5 == 0 || idx == total - 1 {
                let _ = self.tx.send(RepositoryMessage::ScanProgress { 
                    scanned: idx + 1, 
                    total 
                });
            }
        }
        
        // Send completion message
        let _ = self.tx.send(RepositoryMessage::ScanCompleted { 
            total_files: total 
        });
    }

    /// Collect all .pn files in directory tree
    fn collect_pn_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_pn_files_recursive(dir, &mut files);
        files
    }

    /// Helper to recursively collect .pn files
    #[allow(clippy::only_used_in_recursion)]
    fn collect_pn_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.collect_pn_files_recursive(&path, files);
                } else if path.extension().and_then(|s| s.to_str()) == Some("pn") {
                    files.push(path);
                }
            }
        }
    }

    /// Add a file to the document graph
    pub fn add_file_to_graph(&self, file_path: &Path, content: &str) {
        use gdsl::sync_digraph::Node as GraphNode;

        // Parse the file content
        let result = parser::parse_text(content);
        let rope = ropey::Rope::from_str(content);

        // Create URI for the file
        if let Ok(uri) = Url::from_file_path(file_path) {
            let uri = Self::normalize_url_percent_encoding(&uri);

            // Store in document and AST maps
            self.document_map.insert(uri.clone(), rope);
            self.ast_map.insert(uri.clone(), result.ast.clone());

            // Extract wikilinks WITH locations
            let mut wikilinks = vec![];
            Self::gather_wikilinks(&result.ast, &mut wikilinks);

            // Get root URI for link resolution
            if let Ok(root_uri) = Url::from_directory_path(&self.root_dir) {
                // Group links by target URI
                let mut links_by_target: HashMap<Url, Vec<LinkLocation>> = HashMap::new();

                for (link, anchor, location) in &wikilinks {
                    if let Some(link_uri) = self.link_to_uri(link, &root_uri) {
                        let link_loc = LinkLocation {
                            source_line: location.row,
                            source_col_range: (location.span.0, location.span.1),
                            target_anchor: anchor.clone(),
                        };
                        links_by_target
                            .entry(link_uri)
                            .or_default()
                            .push(link_loc);
                    }
                }

                // Update document graph
                if let Ok(mut graph) = self.document_graph.lock() {
                    // Get or create node for this file
                    let node = graph.get(&uri).unwrap_or_else(|| {
                        let n = GraphNode::new(uri.clone(), result.ast.clone());
                        graph.insert(n.clone());
                        n
                    });

                    // Update edges with location data
                    for (link_uri, locations) in &links_by_target {
                        // Create or get target node
                        let target_node = graph.get(link_uri).unwrap_or_else(|| {
                            // Try to get AST from cache, or create placeholder
                            let target_ast = self
                                .ast_map
                                .get(link_uri)
                                .map(|entry| entry.value().clone())
                                .unwrap_or_else(|| {
                                    // Create a placeholder AST
                                    parser::parse_text("").ast
                                });
                            let n = GraphNode::new(link_uri.clone(), target_ast);
                            graph.insert(n.clone());
                            n
                        });

                        // Disconnect old edge and create new one with updated data
                        let _ = node.disconnect(link_uri);
                        node.connect(
                            &target_node,
                            LinkEdge {
                                locations: locations.clone(),
                            },
                        );
                    }

                    // Remove connections that no longer exist
                    let current_targets: HashSet<_> = links_by_target.keys().collect();
                    let edges_to_remove: Vec<_> = node
                        .iter_out()
                        .filter(|edge| !current_targets.contains(&edge.target().key()))
                        .map(|edge| edge.target().key().clone())
                        .collect();

                    for target_uri in edges_to_remove {
                        let _ = node.disconnect(&target_uri);
                    }
                }
            }
        }
    }

    /// Remove a file from the document graph
    fn remove_file_from_graph(&self, file_path: &Path) {
        if let Ok(uri) = Url::from_file_path(file_path) {
            let uri = Self::normalize_url_percent_encoding(&uri);

            // Remove from maps
            self.document_map.remove(&uri);
            self.ast_map.remove(&uri);

            // Remove from graph
            if let Ok(mut graph) = self.document_graph.lock() {
                if let Some(node) = graph.get(&uri) {
                    // Disconnect all edges
                    let outgoing: Vec<_> =
                        node.iter_out().map(|e| e.target().key().clone()).collect();
                    for target in outgoing {
                        let _ = node.disconnect(&target);
                    }

                    let incoming: Vec<_> =
                        node.iter_in().map(|e| e.source().key().clone()).collect();
                    for source in incoming {
                        if let Some(source_node) = graph.get(&source) {
                            let _ = source_node.disconnect(&uri);
                        }
                    }

                    // Remove the node
                    graph.remove(&uri);
                }
            }
        }
    }

    /// Start filesystem watcher for the repository
    pub async fn start_watcher(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::channel(100);
        let watch_dir = self.root_dir.clone();
        let dir_display = watch_dir.display().to_string();
        let watcher_tx = tx.clone();

        // Spawn a blocking task for the file watcher
        tokio::task::spawn_blocking(move || {
            let mut watcher = RecommendedWatcher::new(
                move |result| {
                    if let Ok(event) = result {
                        let _ = watcher_tx.blocking_send(event);
                    }
                },
                Config::default(),
            )
            .unwrap();

            watcher.watch(&watch_dir, RecursiveMode::Recursive).unwrap();
            std::thread::park();
        });

        println!("Repository watching directory: {}", dir_display);

        let pending_changes: Arc<Mutex<HashMap<PathBuf, Instant>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let debounce_duration = Duration::from_millis(10);

        let repo_tx = self.tx.clone();
        let root_dir = self.root_dir.clone();
        let repository = self.clone();

        // Process events from the channel
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if !(event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove()) {
                    continue;
                }
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) != Some("pn") {
                        continue;
                    }

                    if event.kind.is_create() {
                        let Ok(rel_path) = path.strip_prefix(&root_dir) else {
                            continue;
                        };
                        // Read file content and add to graph
                        let Ok(content) = std::fs::read_to_string(&path) else {
                            continue;
                        };
                        repository.add_file_to_graph(&path, &content);

                        let Ok(file_metadata) = std::fs::metadata(&path) else {
                            continue;
                        };
                        let modified = file_metadata
                            .modified()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let created = file_metadata
                            .created()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        //let link_count = repository.count_links_in_file(&path).unwrap_or(0);
                        let link_count = repository.calculate_back_links(&path).len();

                        let metadata = FileMetadata {
                            modified,
                            created,
                            link_count: link_count.try_into().unwrap(),
                        };

                        let _ = repo_tx.send(RepositoryMessage::FileAdded(
                            rel_path.to_path_buf(),
                            metadata,
                        ));
                    } else if event.kind.is_remove() {
                        let Ok(rel_path) = path.strip_prefix(&root_dir) else {
                            continue;
                        };
                        // Remove from graph
                        repository.remove_file_from_graph(&path);
                        let _ = repo_tx
                            .send(RepositoryMessage::FileRemoved(rel_path.to_path_buf()));
                    } else if event.kind.is_modify() && path.is_file() {
                        {
                            let mut changes = pending_changes.lock().unwrap();
                            changes.insert(path.clone(), Instant::now());
                        }

                        let path_clone = path.clone();
                        let pending_changes_clone = Arc::clone(&pending_changes);
                        let repo_tx_clone = repo_tx.clone();
                        let repository_clone = repository.clone();

                        tokio::spawn(async move {
                            sleep(debounce_duration).await;

                            let should_process = {
                                let mut changes = pending_changes_clone.lock().unwrap();
                                if let Some(&last_change) = changes.get(&path_clone) {
                                    let is_latest = Instant::now().duration_since(last_change)
                                        >= debounce_duration;
                                    if is_latest {
                                        changes.remove(&path_clone);
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            };

                            if should_process {
                                let Ok(content) = tokio::fs::read_to_string(&path_clone).await else {
                                    return;
                                };
                                // Update the document graph with new content
                                repository_clone.update_links_in_graph(&path_clone, &content);
                                let metadata = repository_clone.collect_file_metadata(&path_clone).unwrap();

                                // TODO update and broadcast backlinks and two-hop links when other files are created/modified/removed
                                let start = Instant::now();
                                if let Ok(rel_path) = path.strip_prefix(&repository_clone.root_dir) {
                                    // Update the repository's link graph
                                    repository_clone.update_links_in_graph(&path, &content);

                                    let back_links = repository_clone.calculate_back_links(&path);
                                    // Calculate and send back-links and two-hop links for affected files
                                    let _ = repository_clone
                                        .tx
                                        .send(RepositoryMessage::BackLinksChanged(
                                            path.clone(),
                                            back_links,
                                        ));

                                    let two_hop_links = repository_clone.calculate_two_hop_links(&path).await;
                                    let _ = repository_clone
                                        .tx
                                        .send(RepositoryMessage::TwoHopLinksChanged(
                                            path.clone(),
                                            two_hop_links,
                                        ));

                                    println!(
                                        "File {} processed in {} ms",
                                        rel_path.display(),
                                        start.elapsed().as_millis()
                                    );
                                }

                                let _ = repo_tx_clone.send(RepositoryMessage::FileChanged(
                                    path_clone, metadata, content,
                                ));
                            }
                        });
                    }
                }
            }
        });

        Ok(())
    }
}
