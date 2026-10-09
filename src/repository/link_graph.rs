use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use gdsl::sync_digraph::{Graph, Node as GraphNode};
use url::Url;
use urlencoding::encode;

use crate::parser::{self, AstNode, Location};

use super::{BackLinkData, LinkEdge, LinkLocation, LinkLocationData, Repository};

type DocumentGraph = Graph<Url, AstNode, LinkEdge>;
type DocumentNode = GraphNode<Url, AstNode, LinkEdge>;

impl Repository {
    /// Gather wikilinks with their source locations
    pub fn gather_wikilinks(
        parent: &AstNode,
        wikilinks: &mut Vec<(String, Option<String>, Location)>,
    ) {
        crate::ast_query::gather_wikilinks(parent, wikilinks);
    }

    /// The note file a link name refers to, if it exists.
    pub fn link_to_path(&self, link: &str) -> Option<PathBuf> {
        if link.is_empty() {
            return None;
        }
        let file_path = self.root_dir.join(format!("{}.pn", link));
        file_path.exists().then_some(file_path)
    }

    /// The link name of a note file under the repository root.
    pub fn path_to_link(&self, path: &Path) -> Option<String> {
        let rel_path = path.strip_prefix(&self.root_dir).ok()?;
        Some(rel_path.file_stem()?.to_str()?.to_string())
    }

    /// The URI of the note a link name refers to, whether or not it exists.
    pub fn link_to_uri(&self, link: &str, root_uri: &Url) -> Option<Url> {
        if link.is_empty() {
            return None;
        }
        let mut root_path = root_uri.path().to_string();
        if !root_path.ends_with('/') {
            root_path.push('/');
        }
        let mut linkuri = root_uri.clone();
        linkuri.set_path(&format!("{}{}.pn", root_path, encode(link)));
        Some(Self::normalize_url_percent_encoding(&linkuri))
    }

    /// Upper-case every percent escape, so that URIs from editors and from
    /// `Url::from_file_path` compare equal.
    pub fn normalize_url_percent_encoding(url: &Url) -> Url {
        static PERCENT_ESCAPE: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"%[0-9a-fA-F]{2}").unwrap());
        let normalized = PERCENT_ESCAPE.replace_all(url.as_str(), |caps: &regex::Captures| {
            caps[0].to_uppercase()
        });
        Url::parse(&normalized).unwrap_or(url.clone())
    }

    /// The graph key of `file_path`, or `None` for a path outside the repository.
    fn uri_within_root(&self, file_path: &Path) -> Option<Url> {
        let canonical_base = std::fs::canonicalize(&self.root_dir).ok()?;
        let canonical_file = std::fs::canonicalize(file_path).ok()?;
        if !canonical_file.starts_with(&canonical_base) {
            return None;
        }
        let uri = Url::from_file_path(&canonical_file).ok()?;
        Some(Self::normalize_url_percent_encoding(&uri))
    }

    fn link_name_of(&self, uri: &Url) -> Option<String> {
        self.path_to_link(&uri.to_file_path().ok()?)
    }

    /// Every document linking to `file_path`, sorted by link name.
    pub fn calculate_back_links(&self, file_path: &Path) -> Vec<BackLinkData> {
        let Some(uri) = self.uri_within_root(file_path) else {
            return Vec::new();
        };

        let mut result = Vec::new();
        if let Ok(graph) = self.document_graph.lock() {
            for (source_uri, source_node) in graph.iter() {
                for edge in source_node.iter_out() {
                    if edge.target().key() != &uri {
                        continue;
                    }
                    let Some(source_file) = self.link_name_of(source_uri) else {
                        continue;
                    };
                    let locations = edge
                        .value()
                        .locations
                        .iter()
                        .map(|loc| LinkLocationData {
                            line: loc.source_line,
                            col_range: loc.source_col_range,
                            context: None,
                            target_anchor: loc.target_anchor.clone(),
                        })
                        .collect();
                    result.push(BackLinkData {
                        source_file,
                        locations,
                    });
                }
            }
        }

        result.sort_by(|a, b| a.source_file.cmp(&b.source_file));
        result
    }

    /// Number of links pointing at `file_path`, counting each occurrence.
    pub fn count_back_links(&self, file_path: &Path) -> usize {
        let back_links = self.calculate_back_links(file_path);
        back_links.iter().map(|bl| bl.locations.len()).sum()
    }

    /// For each note `file_path` links to, the other notes linking to it as
    /// well, most connected first.
    pub async fn calculate_two_hop_links(&self, file_path: &Path) -> Vec<(String, Vec<String>)> {
        let Some(uri) = self.uri_within_root(file_path) else {
            return Vec::new();
        };

        let mut two_hop_links: Vec<(String, Vec<String>)> = Vec::new();
        if let Ok(graph) = self.document_graph.lock() {
            if let Some(node) = graph.get(&uri) {
                for edge in node.iter_out() {
                    let target_uri = edge.target().key();
                    let mut connected_files: Vec<String> = edge
                        .target()
                        .iter_in()
                        .filter(|incoming| {
                            let source_uri = incoming.source().key();
                            source_uri != &uri && source_uri != target_uri
                        })
                        .filter_map(|incoming| self.link_name_of(incoming.source().key()))
                        .collect();
                    if connected_files.is_empty() {
                        continue;
                    }
                    let Some(bridge_link_name) = self.link_name_of(target_uri) else {
                        continue;
                    };
                    connected_files.sort();
                    two_hop_links.push((bridge_link_name, connected_files));
                }
            }
        }

        two_hop_links.sort_by_key(|(_, connections)| -(connections.len() as i32));
        two_hop_links
    }

    /// Parse `content` as the document at `file_path` and replace its node and
    /// outgoing edges in the graph.
    pub fn add_file_to_graph(&self, file_path: &Path, content: &str) {
        let result = parser::parse_text(content);
        let Ok(uri) = Url::from_file_path(file_path) else {
            return;
        };
        let uri = Self::normalize_url_percent_encoding(&uri);

        self.document_map
            .insert(uri.clone(), ropey::Rope::from_str(content));
        self.ast_map.insert(uri.clone(), result.ast.clone());

        let Ok(root_uri) = Url::from_directory_path(&self.root_dir) else {
            return;
        };
        let links_by_target = self.links_by_target(&result.ast, &root_uri);

        if let Ok(mut graph) = self.document_graph.lock() {
            let node = get_or_insert_node(&mut graph, &uri, || result.ast.clone());
            for (link_uri, locations) in &links_by_target {
                // Skip self-links: connecting a node to itself causes a
                // re-entrancy deadlock in gdsl's RwLock (connect() acquires
                // write on self then write on other, which are the same lock).
                if link_uri == &uri {
                    continue;
                }
                let target_node = get_or_insert_node(&mut graph, link_uri, || {
                    self.ast_map
                        .get(link_uri)
                        .map(|entry| entry.value().clone())
                        .unwrap_or_else(|| parser::parse_text("").ast)
                });
                let _ = node.disconnect(link_uri);
                node.connect(
                    &target_node,
                    LinkEdge {
                        locations: locations.clone(),
                    },
                );
            }

            let stale_targets: Vec<Url> = node
                .iter_out()
                .filter(|edge| !links_by_target.contains_key(edge.target().key()))
                .map(|edge| edge.target().key().clone())
                .collect();
            for target_uri in stale_targets {
                let _ = node.disconnect(&target_uri);
            }
        }
    }

    fn links_by_target(&self, ast: &AstNode, root_uri: &Url) -> HashMap<Url, Vec<LinkLocation>> {
        let mut wikilinks = vec![];
        Self::gather_wikilinks(ast, &mut wikilinks);

        let mut links_by_target: HashMap<Url, Vec<LinkLocation>> = HashMap::new();
        for (link, anchor, location) in &wikilinks {
            if let Some(link_uri) = self.link_to_uri(link, root_uri) {
                links_by_target
                    .entry(link_uri)
                    .or_default()
                    .push(LinkLocation {
                        source_line: location.row,
                        source_col_range: (location.span.0, location.span.1),
                        target_anchor: anchor.clone(),
                    });
            }
        }
        links_by_target
    }

    pub(super) fn remove_file_from_graph(&self, file_path: &Path) {
        let Ok(uri) = Url::from_file_path(file_path) else {
            return;
        };
        let uri = Self::normalize_url_percent_encoding(&uri);

        self.document_map.remove(&uri);
        self.ast_map.remove(&uri);

        let Ok(mut graph) = self.document_graph.lock() else {
            return;
        };
        let Some(node) = graph.get(&uri) else {
            return;
        };

        let outgoing: Vec<_> = node.iter_out().map(|e| e.target().key().clone()).collect();
        for target in outgoing {
            let _ = node.disconnect(&target);
        }

        let incoming: Vec<_> = node.iter_in().map(|e| e.source().key().clone()).collect();
        for source in incoming {
            if let Some(source_node) = graph.get(&source) {
                let _ = source_node.disconnect(&uri);
            }
        }

        graph.remove(&uri);
    }
}

fn get_or_insert_node(
    graph: &mut DocumentGraph,
    uri: &Url,
    ast: impl FnOnce() -> AstNode,
) -> DocumentNode {
    graph.get(uri).unwrap_or_else(|| {
        let node = GraphNode::new(uri.clone(), ast());
        graph.insert(node.clone());
        node
    })
}
