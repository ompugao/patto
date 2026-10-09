import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/note_find.dart';

void main() {
  group('blockIndexForRow', () {
    // Blocks start on rows 0, 1 and 4: the second spans rows 1-3.
    const rows = [0, 1, 4];

    test('a row inside a multi-line block maps to that block', () {
      expect(blockIndexForRow(rows, 1), 1);
      expect(blockIndexForRow(rows, 3), 1);
      expect(blockIndexForRow(rows, 4), 2);
    });

    test('rows past the last block map to it', () {
      expect(blockIndexForRow(rows, 99), 2);
    });

    test('rows before the first block map to it', () {
      expect(blockIndexForRow([2, 5], 0), 0);
    });
  });

  group('findHitBlocks', () {
    test('lists each matching block once, in order, ignoring case', () {
      final lines = ['Tea', 'code', '  tea', '  TEA', 'milk', 'tea'];
      expect(findHitBlocks(lines, [0, 1, 4, 5], 'tea'), [0, 1, 3]);
    });

    test('is empty when nothing matches', () {
      expect(findHitBlocks(['a', 'b'], [0, 1], 'z'), isEmpty);
    });
  });

  group('NoteFind', () {
    test('the count label follows the term and the hits', () {
      expect(NoteFind().countLabel, '');
      expect(NoteFind(term: 'x').countLabel, 'No matches');
      expect(
        NoteFind(term: 'x', hits: [2, 5, 9], current: 1).countLabel,
        '2 / 3',
      );
    });

    test('stepping wraps around the hits', () {
      final find = NoteFind(term: 'x', hits: [2, 5, 9]);
      expect(find.stepped(1).current, 1);
      expect(find.stepped(-1).current, 2);
      expect(find.stepped(1).stepped(1).stepped(1).current, 0);
    });

    test('new hits restart from the first when jumping', () {
      final find = NoteFind(term: 'x', hits: [2, 5, 9], current: 2);
      expect(find.withHits([1, 2], jump: true).current, 0);
    });

    test('new hits keep the position as far as they allow otherwise', () {
      final find = NoteFind(term: 'x', hits: [2, 5, 9], current: 2);
      expect(find.withHits([1, 2, 3, 4], jump: false).current, 2);
      expect(find.withHits([1, 2], jump: false).current, 1);
      expect(find.withHits([], jump: false).current, 0);
    });

    test('the hit set answers whether a block matches', () {
      final find = NoteFind(term: 'x', hits: [2, 5]);
      expect(find.hitSet.contains(5), isTrue);
      expect(find.hitSet.contains(3), isFalse);
    });
  });
}
