; Capture groups follow the nvim-treesitter guidelines documented in
; https://github.com/nvim-treesitter/nvim-treesitter/blob/master/CONTRIBUTING.md

; Links and resources
(expr_wiki_link) @markup.link
(expr_url_link) @markup.link
(expr_local_file_link) @markup.link
(expr_mail_link) @markup.link
(expr_img) @markup.link

(wiki_link) @markup.link.label
(wiki_link_anchored
  (wiki_link) @markup.link.label
  (expr_anchor (anchor) @markup.link.url))
(self_link_anchored
  (expr_anchor (anchor) @markup.link.url))

(URL) @markup.link.url
(url_title) @markup.link.label
(local_file) @markup.link.url
(local_file_title) @markup.link.label
(mail_title) @markup.link.label
(MAIL) @markup.link.url
(img_path) @markup.link.url
(img_body (quoted_string) @markup.link.label)

; Anchors
(expr_anchor) @markup.link
(anchor) @markup.link.label
("#") @punctuation.special

; Commands and properties
(expr_command (builtin_commands) @function)
(builtin_commands) @function
(parameter (identifier) @variable.parameter)
(parameter (quoted_string) @string)
(expr_property) @markup.quote
(property_name) @property
(property_keyword_arg) @property
(property_keyword_value) @string
("{") @punctuation.bracket
("}") @punctuation.bracket
("@") @punctuation.special
("=") @operator

; Inline constructs
(expr_code_inline) @markup.raw
(inline_code_content) @markup.raw
(expr_math_inline) @markup.math
(inline_math_content) @markup.math
(expr_hr) @markup.list

; Decorated text
(builtin_symbol_list) @punctuation.special
((expr_builtin_symbols
   (builtin_symbol_list) @_sym
   (nested_statement) @markup.strong)
 (#match? @_sym "\\*+"))
((expr_builtin_symbols
   (builtin_symbol_list) @_sym
   (nested_statement) @markup.italic)
 (#match? @_sym "/+"))
((expr_builtin_symbols
   (builtin_symbol_list) @_sym
   (nested_statement) @markup.underline)
 (#match? @_sym "_+"))
((expr_builtin_symbols
   (builtin_symbol_list) @_sym
   (nested_statement) @markup.strikethrough)
 (#match? @_sym "-+"))

; Tasks
((expr_task) @markup.list.checked
 (#match? @markup.list.checked "^-"))
((expr_task) @markup.list.unchecked
 (#match? @markup.list.unchecked "^[!*]"))
(task_due) @constant.numeric

; General text
(raw_sentence) @markup.raw
