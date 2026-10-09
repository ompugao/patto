import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/conflicts/conflict_text.dart';
import 'package:patto_flutter/src/rust/api/conflict.dart';

ConflictFile _file(ConflictKind kind, {int conflicts = 0}) => ConflictFile(
  path: 'note.pn',
  kind: kind,
  oursChanged: 2,
  theirsChanged: 3,
  conflicts: conflicts,
);

void main() {
  group('changedRange', () {
    test('is the middle once the shared prefix and suffix are taken off', () {
      expect(changedRange('green tea', 'black tea'), (start: 0, end: 5));
      expect(changedRange('milk done', 'milk todo'), (start: 5, end: 9));
      expect(changedRange('a big cat', 'a cat'), (start: 2, end: 6));
    });

    test('is empty when the lines are the same', () {
      expect(changedRange('same', 'same'), (start: 4, end: 4));
    });

    test('is the whole line when nothing is shared', () {
      expect(changedRange('abc', 'xyz'), (start: 0, end: 3));
    });
  });

  test('tabs are shown as four spaces', () {
    expect(displayLine('\t\tx'), '        x');
  });

  group('conflictFileSubtitle', () {
    test('a note deleted on one side has no conflict count', () {
      expect(
        conflictFileSubtitle(_file(ConflictKind.deletedByThem)),
        'Remote deleted it · you changed 2 lines',
      );
      expect(
        conflictFileSubtitle(_file(ConflictKind.deletedByUs)),
        'You deleted it · remote changed 3 lines',
      );
    });

    test('a note changed on both sides says how many places clash', () {
      expect(
        conflictFileSubtitle(_file(ConflictKind.bothModified)),
        'You: 2 lines · Remote: 3 lines\nmerges cleanly, just confirm',
      );
      expect(
        conflictFileSubtitle(_file(ConflictKind.bothModified, conflicts: 1)),
        'You: 2 lines · Remote: 3 lines\n1 conflict',
      );
      expect(
        conflictFileSubtitle(_file(ConflictKind.bothAdded, conflicts: 4)),
        'Created on both sides\n4 conflicts',
      );
    });
  });
}
