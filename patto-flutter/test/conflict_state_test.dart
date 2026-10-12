import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/conflicts/conflict_state.dart';
import 'package:patto_flutter/src/rust/api/conflict.dart';
import 'package:patto_flutter/src/rust/api/merge.dart';

ConflictDetail _detail(
  List<MergeRegion> regions, {
  ConflictKind kind = ConflictKind.bothModified,
  String? ours = 'ours\n',
  String? theirs = 'theirs\n',
}) => ConflictDetail(
  path: 'note.pn',
  kind: kind,
  oursId: ours == null ? null : 'o',
  theirsId: theirs == null ? null : 't',
  ours: ours,
  theirs: theirs,
  merged: MergedNote(regions: regions, trailingNewline: true),
);

void main() {
  final regions = <MergeRegion>[
    const MergeRegion.unchanged(lines: ['bread']),
    const MergeRegion.conflict(
      base: ['milk {@task status=todo}'],
      ours: ['milk {@task status=done}'],
      theirs: ['oat milk {@task status=todo}'],
      suggestion: Suggestion(
        kind: SuggestionKind.combined,
        lines: ['oat milk {@task status=done}'],
      ),
    ),
    const MergeRegion.ours(base: ['eggs'], lines: ['eggs', 'butter']),
    const MergeRegion.conflict(
      base: ['tea'],
      ours: ['green tea'],
      theirs: ['black tea'],
    ),
  ];

  test('a fresh draft starts every suggested conflict on its suggestion', () {
    final detail = _detail(regions);
    final draft = freshDraft(detail);

    expect(conflictIndices(detail), [1, 3]);
    expect(draft.choices.keys, [1]);
    expect(unresolvedCount(detail, draft), 1);
  });

  test('the merged text follows the choices and undone changes', () {
    final detail = _detail(regions);
    var draft = freshDraft(detail).copyWith(
      choices: {1: const Choice(Pick.suggested), 3: const Choice(Pick.both)},
    );
    expect(
      mergedContent(detail, draft),
      'bread\noat milk {@task status=done}\neggs\nbutter\n'
      'green tea\nblack tea\n',
    );

    draft = draft.copyWith(
      undone: {2},
      choices: {
        1: const Choice(Pick.theirs),
        3: const Choice(Pick.custom, ['chai']),
      },
    );
    expect(
      mergedContent(detail, draft),
      'bread\noat milk {@task status=todo}\neggs\nchai\n',
    );
  });

  group('nextConflictCursor', () {
    const indices = [1, 3, 5];

    test('the first jump lands on the first unresolved conflict', () {
      expect(nextConflictCursor(indices, const {}, -1, 1), 0);
      expect(
        nextConflictCursor(indices, {1: const Choice(Pick.ours)}, -1, 1),
        1,
      );
    });

    test('skips conflicts that already have a choice', () {
      expect(
        nextConflictCursor(indices, {3: const Choice(Pick.ours)}, 0, 1),
        2,
      );
    });

    test('wraps around in both directions', () {
      expect(nextConflictCursor(indices, const {}, 2, 1), 0);
      expect(nextConflictCursor(indices, const {}, 0, -1), 2);
    });

    test('steps to the neighbour once every conflict has a choice', () {
      final all = {for (final i in indices) i: const Choice(Pick.ours)};
      expect(nextConflictCursor(indices, all, 0, 1), 1);
      expect(nextConflictCursor(indices, all, 0, -1), 2);
    });
  });

  test('a note deleted on one side is kept or deleted as a whole', () {
    final detail = _detail(
      const [],
      kind: ConflictKind.deletedByThem,
      ours: 'mine\n',
      theirs: null,
    );
    expect(conflictIndices(detail), [-1]);

    final keep = freshDraft(detail)
        .copyWith(choices: {-1: const Choice(Pick.ours)});
    expect(mergedContent(detail, keep), 'mine\n');

    final delete = freshDraft(detail)
        .copyWith(choices: {-1: const Choice(Pick.theirs)});
    expect(mergedContent(detail, delete), isNull);
  });

  test(
    'a draft survives json and only matches the versions it was made for',
    () {
      final detail = _detail(regions);
      final draft = freshDraft(detail).copyWith(
        choices: {
          3: const Choice(Pick.custom, ['chai']),
        },
        undone: {2},
        done: true,
      );

      final back = NoteDraft.fromJson(
        (jsonDecode(jsonEncode(draft.toJson())) as Map).cast<String, Object?>(),
      );
      expect(back.choices[3]!.pick, Pick.custom);
      expect(back.choices[3]!.custom, ['chai']);
      expect(back.undone, {2});
      expect(back.done, isTrue);
      expect(back.matches(detail), isTrue);
      expect(
        back.matches(_detail(regions, ours: 'edited\n')),
        isTrue,
        reason: 'ids, not texts, identify a version',
      );

      final changed = ConflictDetail(
        path: detail.path,
        kind: detail.kind,
        oursId: 'o2',
        theirsId: detail.theirsId,
        ours: detail.ours,
        theirs: detail.theirs,
        merged: detail.merged,
      );
      expect(back.matches(changed), isFalse);
    },
  );
}
