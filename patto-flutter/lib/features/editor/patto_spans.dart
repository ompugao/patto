import 'package:flutter/material.dart';

/// Styling of a raw patto line in the editor.
///
/// The span must spell out the line exactly, since the editor maps caret
/// offsets through it, so everything here is colour and spacing only.

/// Flutter draws a tab as wide as a space, which makes nesting nearly
/// invisible in a monospace font; letter spacing on the tab widens it without
/// touching the text.
const double tabExtraEm = 0.9;

enum PattoToken { command, code, math, decoration, link, property, task, anchor }

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
      guides = [
        scheme.onSurface.withValues(alpha: 0.05),
        scheme.onSurface.withValues(alpha: 0.10),
      ],
      activeGuide = scheme.primary.withValues(alpha: 0.30),
      verbatim = TextStyle(color: scheme.tertiary);

  final ColorScheme _scheme;

  /// Background of each indent column, alternating so that neighbouring
  /// levels stay apart.
  final List<Color> guides;

  /// Background of the column the caret's block hangs from.
  final Color activeGuide;

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
/// [activeColumn] is the indent column to emphasise on this line, if any;
/// [verbatim] marks a line inside a code or math block, whose text is not
/// patto markup.
TextSpan pattoLineSpan({
  required String text,
  required TextStyle style,
  required PattoSpanStyles styles,
  bool verbatim = false,
  int? activeColumn,
}) {
  if (text.isEmpty) return TextSpan(text: text, style: style);
  final children = <TextSpan>[];

  var depth = 0;
  while (depth < text.length && text.codeUnitAt(depth) == 0x09) {
    depth++;
  }
  final tabSpacing = (style.fontSize ?? 14) * tabExtraEm;
  for (var i = 0; i < depth; i++) {
    children.add(
      TextSpan(
        text: '\t',
        style: TextStyle(
          letterSpacing: tabSpacing,
          backgroundColor: i == activeColumn
              ? styles.activeGuide
              : styles.guides[i % styles.guides.length],
        ),
      ),
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
      if (t.start > at) children.add(TextSpan(text: content.substring(at, t.start)));
      final piece = content.substring(t.start, t.end);
      children.add(TextSpan(text: piece, style: styles.token(t.kind, piece)));
      at = t.end;
    }
    if (at < content.length) children.add(TextSpan(text: content.substring(at)));
  }

  return TextSpan(style: style, children: children);
}
