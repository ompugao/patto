; Injection queries for patto code blocks
; Inject language grammar into code block content lines

; Note: In the flat grammar, code block content is just indented `line` nodes
; following a `command_line` with `[@code lang]`. Injection via tree-sitter
; requires block structure which this flat grammar doesn't provide.
; Language injection for code blocks is handled by the LSP/editor plugin.
