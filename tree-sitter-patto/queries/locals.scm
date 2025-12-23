; Locals capture names follow https://github.com/nvim-treesitter/nvim-treesitter/blob/master/CONTRIBUTING.md

(statement
  (expr_anchor (anchor) @local.definition))

(trailing_properties
  (expr_anchor (anchor) @local.definition))

(expr_wiki_link
  (wiki_link) @local.reference)

(wiki_link_anchored
  (wiki_link) @local.reference
  (expr_anchor (anchor) @local.reference))

(self_link_anchored
  (expr_anchor (anchor) @local.reference))
