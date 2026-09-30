import 'package:flutter/material.dart';

/// Styling of a raw patto line in the editor.
///
/// The span must spell out the line exactly, since the editor maps caret
/// offsets through it, so everything here is colour and spacing only.

/// Flutter draws a tab as wide as a space, which makes nesting nearly
/// invisible in a monospace font. Drawing the tab in a larger font widens it
/// without touching the text; the line height is fixed by the editor's strut,
/// so it does not grow. Letter spacing would also widen it, but pads the tab
/// on both sides and shifts the whole line right.
const double tabFontScale = 2.5;

/// The style of a leading tab on a line in [style].
TextStyle tabStyle(TextStyle style) =>
    TextStyle(fontSize: (style.fontSize ?? 14) * tabFontScale);

enum PattoToken {
  command,
  code,
  math,
  decoration,
  link,
  property,
  task,
  anchor,
}

final _tokenPattern = RegExp(
  r'(?<code>\[`.*?`\])'
  r'|(?<math>\[\$.*?\$\])'
  r'|(?<command>\[@[a-z]+(?:\s[^\]]*)?\])'
  r'|(?<decoration>\[[*/_\-]+\s[^\[\]]*\])'
  r'|(?<link>\[[^\[\]]+\])'
  r'|(?<property>\{@[^{}]*\})'
  r'|(?<task>(?<=^|\s)[!*\-]\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2})?(?=\s|$))'
  r'|(?<anchor>(?<=^|\s)#[^\s\]}]+)',
);

/// Marked-up ranges of [text], in order and without overlaps.
List<({int start, int end, PattoToken kind})> tokenize(String text) {
  return [
    for (final m in _tokenPattern.allMatches(text))
      (
        start: m.start,
        end: m.end,
        kind: PattoToken.values.firstWhere((k) => m.namedGroup(k.name) != null),
      ),
  ];
}

/// Whether a line opens a block whose children are taken verbatim.
bool opensVerbatim(String line) {
  final content = line.trimLeft();
  return content.startsWith('[@code') || content.startsWith('[@math');
}

class PattoSpanStyles {
  PattoSpanStyles(ColorScheme scheme)
    : _scheme = scheme,
      verbatim = TextStyle(color: scheme.tertiary);

  final ColorScheme _scheme;

  final TextStyle verbatim;

  TextStyle token(PattoToken kind, String text) => switch (kind) {
    PattoToken.command => TextStyle(
      color: _scheme.primary,
      fontWeight: FontWeight.bold,
    ),
    PattoToken.code => TextStyle(
      color: _scheme.tertiary,
      backgroundColor: _scheme.surfaceContainerHighest,
    ),
    PattoToken.math => TextStyle(color: _scheme.tertiary),
    PattoToken.decoration => _decoration(text),
    PattoToken.link => TextStyle(color: _scheme.primary),
    PattoToken.property => TextStyle(color: _scheme.onSurfaceVariant),
    PattoToken.task => TextStyle(
      color: switch (text[0]) {
        '!' => _scheme.error,
        '*' => _scheme.primary,
        _ => _scheme.outline,
      },
    ),
    PattoToken.anchor => TextStyle(color: _scheme.secondary),
  };

  TextStyle _decoration(String text) {
    final symbols = text.substring(1, text.indexOf(RegExp(r'\s')));
    return TextStyle(
      fontWeight: symbols.contains('*') ? FontWeight.bold : null,
      fontStyle: symbols.contains('/') ? FontStyle.italic : null,
      decoration: TextDecoration.combine([
        if (symbols.contains('_')) TextDecoration.underline,
        if (symbols.contains('-')) TextDecoration.lineThrough,
      ]),
    );
  }
}

/// The span for one editor line.
///
/// [verbatim] marks a line inside a code or math block, whose text is not
/// patto markup.
TextSpan pattoLineSpan({
  required String text,
  required TextStyle style,
  required PattoSpanStyles styles,
  bool verbatim = false,
}) {
  if (text.isEmpty) return TextSpan(text: text, style: style);
  final children = <TextSpan>[];

  var depth = 0;
  while (depth < text.length && text.codeUnitAt(depth) == 0x09) {
    depth++;
  }
  if (depth > 0) {
    children.add(
      TextSpan(text: text.substring(0, depth), style: tabStyle(style)),
    );
  }

  final content = text.substring(depth);
  if (verbatim) {
    if (content.isNotEmpty) {
      children.add(TextSpan(text: content, style: styles.verbatim));
    }
  } else {
    var at = 0;
    for (final t in tokenize(content)) {
      if (t.start > at) {
        children.add(TextSpan(text: content.substring(at, t.start)));
      }
      final piece = content.substring(t.start, t.end);
      children.add(TextSpan(text: piece, style: styles.token(t.kind, piece)));
      at = t.end;
    }
    if (at < content.length) {
      children.add(TextSpan(text: content.substring(at)));
    }
  }

  return TextSpan(style: style, children: children);
}
