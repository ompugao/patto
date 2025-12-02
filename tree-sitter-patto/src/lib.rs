use tree_sitter::Language;

extern "C" {
    fn tree_sitter_patto() -> Language;
}

/// Returns the tree-sitter [`Language`] for the Patto grammar.
pub fn language() -> Language {
    unsafe { tree_sitter_patto() }
}

/// Returns the JSON description of the syntax tree nodes produced by this grammar.
pub fn node_types_json() -> &'static str {
    include_str!("node-types.json")
}

/// Returns the JSON representation of the grammar itself.
pub fn grammar_json() -> &'static str {
    include_str!("grammar.json")
}
