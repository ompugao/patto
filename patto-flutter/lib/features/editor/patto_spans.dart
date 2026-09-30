import 'package:flutter/material.dart';

import 'outline.dart';

// Styling of a raw patto line in the editor. A span must spell out its line
// exactly, since the editor maps caret offsets through it.

/// Flutter draws a tab one space wide, which hides nesting. A larger font
/// widens it without touching the text, and the editor's fixed line height
/// keeps the line from growing. (Letter spacing pads both sides of the tab
/// and shifts the line right.)
TextStyle _tabStyle(TextStyle style) =>
    TextStyle(fontSize: (style.fontSize ?? 14) * 2.5);

/// How wide a leading tab is drawn in [style].
double tabWidth(TextStyle style) {
  final painter = TextPainter(
    text: TextSpan(text: '\t', style: style.merge(_tabStyle(style))),
    textDirection: TextDirection.ltr,
  )..layout();
  final width = painter.width;
  painter.dispose();
  return width;
}

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

/// The span for one editor line; [verbatim] lines, inside a code or math
/// block, are not patto markup.
TextSpan pattoLineSpan({
  required String text,
  required TextStyle style,
  required PattoSpanStyles styles,
  bool verbatim = false,
}) {
  if (text.isEmpty) return TextSpan(text: text, style: style);
  final children = <TextSpan>[];

  final depth = depthOf(text);
  if (depth > 0) {
    children.add(
      TextSpan(text: text.substring(0, depth), style: _tabStyle(style)),
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
