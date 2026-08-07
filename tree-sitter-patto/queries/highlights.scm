; Highlight queries for patto (nvim-treesitter)

; Commands
(command "[@" @punctuation.special)
(command "]" @punctuation.special)
(command_name) @keyword

; Parameters
(param_word) @string
(key_value_param "=" @operator)
(escaped_string "\"" @string)
(string_content) @string

; Bracket expressions
(bracket_expr "[" @punctuation.bracket)
(bracket_expr "]" @punctuation.bracket)

; Link types
(bracket_url) @markup.link.url
(bracket_mail) @markup.link.url
(bracket_local_file) @markup.link
(bracket_hash) @markup.link

; Decorations
(bracket_decoration_markers) @markup.bold

; Inline code
(code_inline "[`" @punctuation.special)
(code_inline "`]" @punctuation.special)
(code_inline_content) @markup.raw

; Inline math
(math_inline "[$" @punctuation.special)
(math_inline "$]" @punctuation.special)
(math_inline_content) @markup.raw

; Embed
(embed "[@embed" @keyword)
(embed "]" @punctuation.special)

; Image
(image "[@img" @keyword)
(image "]" @punctuation.special)

; Horizontal rule
(horizontal_rule) @punctuation.special

; Anchor
(anchor) @markup.link

; Task
(task) @markup.list

; Property
(property "{@" @punctuation.special)
(property "}" @punctuation.special)
(property_name) @type
(property_kv "=" @operator)
(property_word) @string

; Text
(text_segment) @text
(text_special) @text
(bracket_text) @text
