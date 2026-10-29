import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/block_source.dart';

void main() {
  const lines = [
    'parent',
    '\tchild',
    '[@code dart]',
    '\tvoid main() {}',
    '\t\tprint(1);',
    '',
    'last line',
    '',
  ];
  const rows = [0, 1, 2, 5, 6];

  test('a line with children copies only its own row', () {
    expect(blockSource(lines, rows, 0), 'parent');
  });

  test('a code block copies all of its rows', () {
    expect(
      blockSource(lines, rows, 2),
      '[@code dart]\n\tvoid main() {}\n\t\tprint(1);',
    );
  });

  test(
    'the last block runs to the end of the note without trailing blanks',
    () {
      expect(blockSource(lines, rows, 4), 'last line');
    },
  );

  test('a blank block copies an empty string', () {
    expect(blockSource(lines, rows, 3), '');
  });

  test('a range of blocks copies every row between them', () {
    expect(
      blockSource(lines, rows, 1, 3),
      '\tchild\n[@code dart]\n\tvoid main() {}\n\t\tprint(1);',
    );
  });

  test('a block past the end of stale source copies nothing', () {
    expect(blockSource(['only'], [0, 3], 1), '');
  });
}
