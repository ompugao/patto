import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/outline.dart';
import 'package:patto_flutter/features/editor/patto_spans.dart';
import 'package:re_editor/re_editor.dart';

const _note = [
  'root', //         0
  '\tchild a', //    1
  '\t\tgrand a', //  2
  '', //             3
  '\t\tgrand b', //  4
  '\tchild b', //    5
  '', //             6
  'next', //         7
];

void main() {
  group('blocks', () {
    test('a block takes deeper lines and the blank lines between them', () {
      expect(blockEnd(_note, 0), 6);
      expect(blockEnd(_note, 1), 5);
      expect(blockEnd(_note, 2), 3);
      expect(blockEnd(_note, 5), 6);
      expect(blockEnd(_note, 7), 8);
    });

    test('parents and ancestors', () {
      expect(parentOf(_note, 0), isNull);
      expect(parentOf(_note, 4), 1);
      expect(parentOf(_note, 5), 0);
      expect(parentOf(_note, 3), isNull);
      expect(ancestorsOf(_note, 4), [0, 1]);
    });

    test('the active guide is the own children column, else the siblings one', () {
      expect(activeGuide(_note, 1), (column: 1, start: 2, end: 5));
      expect(activeGuide(_note, 5), (column: 0, start: 1, end: 6));
      expect(activeGuide(_note, 7), isNull);
      expect(activeGuide(_note, 3), isNull);
    });
  });

  group('moveBlock', () {
    List<String> apply(({List<int> order, int row}) moved) =>
        [for (final i in moved.order) _note[i]];

    test('swaps a block with the sibling above, children included', () {
      final moved = moveBlock(_note, 5, up: true)!;
      expect(apply(moved), [
        'root',
        '\tchild b',
        '\tchild a',
        '\t\tgrand a',
        '',
        '\t\tgrand b',
        '',
        'next',
      ]);
      expect(moved.row, 1);
    });

    test('keeps the blank lines between the swapped blocks in place', () {
      final moved = moveBlock(_note, 0, up: false)!;
      expect(apply(moved).first, 'next');
      expect(apply(moved)[1], '');
      expect(moved.row, 2);
    });

    test('does not leave the parent', () {
      expect(moveBlock(_note, 1, up: true), isNull);
      expect(moveBlock(_note, 5, up: false), isNull);
      expect(moveBlock(_note, 4, up: false), isNull);
    });
  });

  test('folds every line with children', () {
    final chunks = const PattoIndentChunkAnalyzer().run(
      CodeLines.fromText(_note.join('\n')),
    );
    expect(chunks, [const CodeChunk(0, 6), const CodeChunk(1, 5)]);
  });

  group('spans', () {
    final styles = PattoSpanStyles(const ColorScheme.light());
    const base = TextStyle(fontSize: 14);

    List<(String, PattoToken)> tokens(String text) => [
      for (final t in tokenize(text)) (text.substring(t.start, t.end), t.kind),
    ];

    test('recognises patto markup', () {
      expect(tokens('see [note#sec] and [`a[0]`] {@task status=todo}'), [
        ('[note#sec]', PattoToken.link),
        ('[`a[0]`]', PattoToken.code),
        ('{@task status=todo}', PattoToken.property),
      ]);
      expect(tokens('[@code python]'), [('[@code python]', PattoToken.command)]);
      expect(tokens('[* bold] [\$x^2\$]'), [
        ('[* bold]', PattoToken.decoration),
        ('[\$x^2\$]', PattoToken.math),
      ]);
      expect(tokens('do it !2026-01-02 #here'), [
        ('!2026-01-02', PattoToken.task),
        ('#here', PattoToken.anchor),
      ]);
      expect(tokens('x-2026-01-02 a#b'), isEmpty);
    });

    test('spell out the line exactly', () {
      for (final line in [..._note, '\t\t[@code] [a] {@x} !2026-01-01', '\t']) {
        final span = pattoLineSpan(text: line, style: base, styles: styles);
        expect(span.toPlainText(), line);
      }
      final verbatim = pattoLineSpan(
        text: '\t\tint a[2];',
        style: base,
        styles: styles,
        verbatim: true,
      );
      expect(verbatim.toPlainText(), '\t\tint a[2];');
    });

    test('draw a tab wider than a space', () {
      double width(String text) {
        final painter = TextPainter(
          text: pattoLineSpan(text: text, style: base, styles: styles),
          textDirection: TextDirection.ltr,
        )..layout();
        return painter.width;
      }

      expect(width('\tx'), greaterThan(width(' x') + 10));
    });
  });
}
