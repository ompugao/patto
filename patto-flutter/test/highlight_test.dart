import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/search/highlight.dart';

const _mark = TextStyle(backgroundColor: Color(0xFFFFFF00));

/// The (text, highlighted) pieces of a span, in order.
List<(String, bool)> _pieces(InlineSpan span) {
  final s = span as TextSpan;
  if (s.children == null) return [(s.text!, false)];
  return [
    for (final c in s.children!.cast<TextSpan>())
      (c.text!, identical(c.style, _mark)),
  ];
}

void main() {
  test('marks every occurrence, ignoring case', () {
    expect(_pieces(highlightedSpan('Foo bar FOO', 'foo', highlight: _mark)), [
      ('Foo', true),
      (' bar ', false),
      ('FOO', true),
    ]);
  });

  test('leaves text alone without a term or a match', () {
    expect(_pieces(highlightedSpan('abc', null, highlight: _mark)), [('abc', false)]);
    expect(_pieces(highlightedSpan('abc', '  ', highlight: _mark)), [('abc', false)]);
    expect(_pieces(highlightedSpan('abc', 'x', highlight: _mark)), [('abc', false)]);
  });

  test('works on Japanese text', () {
    expect(_pieces(highlightedSpan('今日の目的と目的', '目的', highlight: _mark)), [
      ('今日の', false),
      ('目的', true),
      ('と', false),
      ('目的', true),
    ]);
  });
}
