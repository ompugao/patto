import 'package:flutter/painting.dart';

/// Split [text] into spans, giving every case-insensitive occurrence of [term]
/// the [highlight] style on top of [style].
///
/// Returns a single plain span when there is nothing to highlight.
InlineSpan highlightedSpan(
  String text,
  String? term, {
  TextStyle? style,
  required TextStyle highlight,
}) {
  final needle = term?.trim().toLowerCase() ?? '';
  final lower = text.toLowerCase();
  // Lowercasing a few characters changes their length, which would misalign
  // the offsets below; such text is shown without highlights.
  if (needle.isEmpty || lower.length != text.length) {
    return TextSpan(text: text, style: style);
  }

  final children = <TextSpan>[];
  var start = 0;
  while (true) {
    final at = lower.indexOf(needle, start);
    if (at < 0) break;
    if (at > start) {
      children.add(TextSpan(text: text.substring(start, at)));
    }
    children.add(
      TextSpan(text: text.substring(at, at + needle.length), style: highlight),
    );
    start = at + needle.length;
  }
  if (children.isEmpty) {
    return TextSpan(text: text, style: style);
  }
  if (start < text.length) {
    children.add(TextSpan(text: text.substring(start)));
  }
  return TextSpan(style: style, children: children);
}

/// Whether [text] contains [term], ignoring case.
bool containsIgnoringCase(String text, String term) =>
    text.toLowerCase().contains(term.trim().toLowerCase());
