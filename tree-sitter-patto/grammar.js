const PREC = {
  raw: -1,
  command: 1,
};

module.exports = grammar({
  name: 'patto',

  extras: $ => [],

  externals: $ => [
    $._newline,
    $._indent,
    $._dedent,
  ],

  conflicts: $ => [
    [$.url_title, $.mail_title]
  ],

  rules: {
    document: $ => seq(
      repeat(choice($.blank_line, $.line_with_newline)),
      optional($.line)
    ),

    line_with_newline: $ => seq(
      $.line,
      $._newline,
      optional($.block_body)
    ),

    line: $ => $.statement,

    blank_line: $ => $._newline,

    block_body: $ => seq(
      $._indent,
      repeat(choice($.blank_line, $.line_with_newline)),
      optional($.line),
      $._dedent
    ),

    statement: $ => seq(
      repeat(choice($.expr_anchor, $.expr_task)),
      repeat1(choice(
        $.expr_command,
        $.expr_img,
        $.expr_builtin_symbols,
        $.expr_code_inline,
        $.expr_math_inline,
        $.expr_wiki_link,
        $.expr_url_link,
        $.expr_local_file_link,
        $.expr_mail_link,
        $.expr_property,
        $.expr_hr,
        $.raw_sentence
      )),
      optional($.trailing_properties)
    ),

    _WHITE_SPACE_INLINE: _ => token(/[ \t]/),

    raw_sentence: _ => token(prec(PREC.raw, /[^\[\]{}\n]+/)),

    trailing_properties: $ => repeat1(seq(
      repeat1($._WHITE_SPACE_INLINE),
      choice($.expr_property, $.expr_anchor, $.expr_task)
    )),

    expr_command: $ => prec(PREC.command, seq(
      '[', '@', $.builtin_commands,
      repeat(seq(repeat1($._WHITE_SPACE_INLINE), $.parameter)),
      repeat($._WHITE_SPACE_INLINE),
      ']'
    )),

    builtin_commands: $ => choice($.code_command, $.math_command, $.quote_command, $.table_command),
    code_command: $ => 'code',
    math_command: $ => 'math',
    quote_command: $ => 'quote',
    table_command: $ => 'table',

    parameter: $ => choice(
      seq($.identifier, '=', $.quoted_string),
      $.quoted_string,
      $.identifier
    ),

    identifier: _ => token(/[A-Za-z0-9\p{L}\-/:_]+/u),

    quoted_string: _ => token(/"([^"\\]|\\.)*"/),

    expr_img: $ => seq('[', '@img', repeat1($._WHITE_SPACE_INLINE), $.img_body, ']'),

    img_body: $ => choice(
      seq($.quoted_string, repeat1($._WHITE_SPACE_INLINE), $.img_path),
      seq($.img_path, repeat1($._WHITE_SPACE_INLINE), $.quoted_string),
      $.img_path
    ),

    img_path: $ => choice($.URL, $.local_file),

    local_file: _ => token(/([\w\p{L}\._\- ]+\/)+[\w\p{L}\._\- ]+\.[A-Za-z0-9._-]+/u),

    expr_builtin_symbols: $ => seq(
      '[', $.builtin_symbol_list, repeat1($._WHITE_SPACE_INLINE), $.nested_statement, ']'
    ),

    builtin_symbol_list: $ => repeat1(choice('*', '/', '_', '-')),

    nested_statement: $ => repeat1(choice(
      $.expr_code_inline,
      $.expr_math_inline,
      $.expr_wiki_link,
      $.expr_url_link,
      $.expr_local_file_link,
      $.expr_mail_link,
      $.raw_sentence
    )),

    expr_wiki_link: $ => seq('[', choice($.wiki_link_anchored, $.wiki_link, $.self_link_anchored), ']'),
    wiki_link_anchored: $ => seq($.wiki_link, $.expr_anchor),
    wiki_link: _ => token(/[^@`$\[#\]\n][^\[#\]\n]*/),
    self_link_anchored: $ => $.expr_anchor,

    expr_url_link: $ => seq('[', choice($.expr_title_url, $.expr_url_title, $.expr_url_only, $.expr_url_url), ']'),
    expr_title_url: $ => seq($.url_title, repeat1($._WHITE_SPACE_INLINE), $.URL),
    expr_url_title: $ => seq($.URL, repeat1($._WHITE_SPACE_INLINE), $.url_title),
    expr_url_only: $ => $.URL,
    expr_url_url: $ => seq($.URL, repeat1($._WHITE_SPACE_INLINE), $.URL),

    url_title: _ => token(/[^@`$#\[\]\s]+(?:\s+[^\[\]\s#`$]+)*/),
    URL: _ => token(/[A-Za-z]+:\/\/[A-Za-z0-9\p{L}:/#%$&?@!()~.=+*_\-]+/u),

    expr_local_file_link: $ => seq('[', choice($.expr_local_file_title, $.expr_title_local_file, $.expr_local_file_only), ']'),
    expr_local_file_title: $ => seq($.local_file, repeat1($._WHITE_SPACE_INLINE), $.local_file_title),
    expr_title_local_file: $ => seq($.local_file_title, repeat1($._WHITE_SPACE_INLINE), $.local_file),
    expr_local_file_only: $ => $.local_file,
    local_file_title: _ => token(/[^@\[\] #]+(?: [^\[\] #]+)*/),

    expr_mail_link: $ => seq('[', choice($.expr_title_mail, $.expr_mail_title, $.expr_mail_only, $.expr_mail_mail), ']'),
    expr_mail_title: $ => seq($.MAIL, repeat1($._WHITE_SPACE_INLINE), $.mail_title),
    expr_title_mail: $ => seq($.mail_title, repeat1($._WHITE_SPACE_INLINE), $.MAIL),
    expr_mail_only: $ => $.MAIL,
    expr_mail_mail: $ => seq($.MAIL, repeat1($._WHITE_SPACE_INLINE), $.MAIL),
    mail_title: _ => token(/[^@\[\]\s#]+(?:\s+[^\[\]\s#]+)*/),
    MAIL: _ => token(/mailto:[A-Za-z0-9_+\-]+@[A-Za-z0-9_+\-]+(?:\.[A-Za-z0-9_+\-]+)+/),

    expr_code_inline: $ => seq('[', '`', repeat($._WHITE_SPACE_INLINE), $.inline_code_content, '`', ']'),
    inline_code_content: _ => token(/[^`\]]+/),

    expr_math_inline: $ => seq('[', '$', repeat($._WHITE_SPACE_INLINE), $.inline_math_content, '$', ']'),
    inline_math_content: _ => token(/[^$\]]+/),

    expr_property: $ => seq('{', '@', $.property_name, repeat(seq(repeat1($._WHITE_SPACE_INLINE), $.property_assignment)), '}'),
    property_name: _ => token(/[A-Za-z0-9]+/),
    property_assignment: $ => seq($.property_keyword_arg, '=', $.property_keyword_value),
    property_keyword_arg: _ => token(/[A-Za-z0-9]+/),
    property_keyword_value: _ => token(/[A-Za-z0-9\p{L}\-/:_]+/u),

    expr_anchor: $ => seq('#', $.anchor),
    anchor: _ => token(/[A-Za-z0-9\p{L}_\-]+/u),

    expr_task: $ => seq(choice('!', '*', '-'), $.task_due),
    task_due: _ => token(/\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?/),

    expr_hr: _ => token(/-{5,}/),
  }
});
