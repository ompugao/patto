/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

// Tree-sitter grammar for the Patto note-taking format.
// Flat line-oriented: each line parsed independently.
// External scanner emits NEWLINE at \n boundaries.

module.exports = grammar({
  name: 'patto',

  externals: $ => [
    $._newline,
  ],

  extras: _ => [/\t/],

  rules: {
    document: $ => seq(
      repeat(choice(
        seq($._statement, $._newline),
        $._newline,
      )),
      optional($._statement),
    ),

    _statement: $ => choice(
      $.command_line,
      $.line,
    ),

    line: $ => repeat1(choice($._inline, $._inline_ws)),

    command_line: $ => $.command,

    // Block commands
    command: $ => seq(
      '[@',
      $.command_name,
      optional(seq($._ws, $.parameters)),
      ']',
    ),

    command_name: _ => choice('code', 'math', 'quote', 'table'),

    parameters: $ => seq(
      $.parameter,
      repeat(seq($._ws, $.parameter)),
    ),

    parameter: $ => choice($.key_value_param, $.escaped_string, $.bare_param),

    key_value_param: $ => prec(1, seq(
      $.param_word, '=', choice($.escaped_string, $.param_word),
    )),

    bare_param: $ => $.param_word,

    param_word: _ => /[a-zA-Z0-9\u3000-\u9FFF\u3040-\u309F\u30A0-\u30FF\uAC00-\uD7AFー\-\/:_.]+/,

    escaped_string: $ => seq('"', optional($.string_content), '"'),
    string_content: _ => /([^"\\]|\\.)*/,

    // Inline elements
    _inline: $ => choice(
      $.embed,
      $.image,
      $.code_inline,
      $.math_inline,
      $.bracket_expr,
      $.property,
      $.task,
      $.anchor,
      $.horizontal_rule,
      $.text,
    ),

    // Spaces between inline elements on the same line
    _inline_ws: _ => / +/,

    // [...] constructs: wiki links, url links, decorations, etc.
    bracket_expr: $ => seq('[', $.bracket_content, ']'),

    bracket_content: $ => repeat1(choice(
      $.bracket_url,
      $.bracket_mail,
      $.bracket_local_file,
      $.bracket_hash,
      $.bracket_decoration_markers,
      $.bracket_text,
      $.bracket_ws,
    )),

    bracket_url: _ => token(prec(3, /[a-zA-Z]+:\/\/[a-zA-Z0-9\u3000-\u9FFF\u3040-\u309F\u30A0-\u30FF\uAC00-\uD7AF\/:@#%$&?!()~.=+*\-_]+/)),
    bracket_mail: _ => token(prec(3, /mailto:[a-zA-Z0-9\-_+]+@([a-zA-Z0-9\-_]+\.)+[a-zA-Z0-9\-_]+/)),
    bracket_local_file: _ => token(prec(2, /([a-zA-Z0-9_\-.]+\/)+[a-zA-Z0-9_\-]+(\.[a-zA-Z]+)+/)),
    bracket_hash: _ => token(prec(1, /#[^ \t\[\]{}\n\r]+/)),
    bracket_decoration_markers: _ => token(prec(2, /[*/_\-]+ /)),
    bracket_text: _ => token(prec(-1, /[^\[\]\n\r\t# ]+/)),
    bracket_ws: _ => /[ ]+/,

    embed: $ => seq('[@embed', $._ws, $.embed_content, ']'),
    embed_content: $ => repeat1(choice($.bracket_url, $.bracket_text, $.bracket_ws)),

    image: $ => seq('[@img', $._ws, $.image_content, ']'),
    image_content: $ => repeat1(choice($.bracket_url, $.bracket_local_file, $.escaped_string, $.bracket_text, $.bracket_ws)),

    code_inline: $ => seq('[`', optional($._ws), optional($.code_inline_content), '`]'),
    code_inline_content: _ => /[^`\n\r]+/,

    math_inline: $ => seq('[$', optional($._ws), optional($.math_inline_content), '$]'),
    math_inline_content: _ => /[^$\n\r]+/,

    // Horizontal rule: 5+ dashes
    horizontal_rule: _ => token(prec(5, /-----[-]*/)),

    // Anchor: #name (requires at least one name char after #)
    anchor: _ => token(/#[^ \t\[\]{}\n\r#]+/),

    // Task: marker immediately followed by date (single token to avoid conflicts)
    task: _ => token(prec(1, /[!*\-][0-9]{4}-[0-9]{2}-[0-9]{2}(T[0-9]{2}:[0-9]{2})?/)),

    // Property: {@name key=value ...}
    property: $ => seq(
      '{@', $.property_name,
      repeat(seq($._ws, choice($.property_kv, $.property_word))),
      '}',
    ),

    property_name: _ => /[a-zA-Z0-9_]+/,
    property_kv: $ => prec(1, seq($.property_word, '=', $.property_word)),
    property_word: _ => /[a-zA-Z0-9\u3000-\u9FFF\u3040-\u309F\u30A0-\u30FF\uAC00-\uD7AF\-\/:_T]+/,

    // Text (catch-all)
    text: $ => prec(-1, repeat1(choice($.text_segment, $.text_special))),
    text_segment: _ => token(prec(-2, /[^\[\]{}\n\r\t !*\-#]+/)),
    // Bare special chars that didn't match task/anchor/horizontal_rule
    text_special: _ => token(prec(-3, /[!*\-#]+/)),

    _ws: _ => /[ \t]+/,
  },
});
