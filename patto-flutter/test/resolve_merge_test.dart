import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/conflicts/conflict_state.dart';
import 'package:patto_flutter/features/conflicts/resolve_merge.dart';
import 'package:patto_flutter/src/rust/api/conflict.dart';
import 'package:patto_flutter/src/rust/api/merge.dart';

const _regions = <MergeRegion>[
  MergeRegion.unchanged(lines: ['bread']),
  MergeRegion.conflict(base: ['tea'], ours: ['green tea'], theirs: ['chai']),
];

ConflictDetail _detail(String path, {String oursId = 'o'}) => ConflictDetail(
  path: path,
  kind: ConflictKind.bothModified,
  oursId: oursId,
  theirsId: 't',
  ours: 'ours\n',
  theirs: 'theirs\n',
  merged: const MergedNote(regions: _regions, trailingNewline: true),
);

PendingConflict _pending(List<String> paths) => PendingConflict(
  remote: const RemoteCommit(id: 'r', summary: '', author: '', timeMs: 0),
  sideBranch: 'patto/side',
  files: [
    for (final p in paths)
      ConflictFile(
        path: p,
        kind: ConflictKind.bothModified,
        oursChanged: 1,
        theirsChanged: 1,
        conflicts: 1,
      ),
  ],
  heldBack: const [],
);

void main() {
  test('a finished draft becomes the merged content of its note', () async {
    final detail = _detail('a.pn');
    final draft = freshDraft(detail)
        .copyWith(choices: {1: const Choice(Pick.theirs)}, done: true);

    final resolutions = await collectResolutions(_pending(['a.pn']), {
      'a.pn': draft,
    }, (_) async => detail);

    expect(resolutions.single.path, 'a.pn');
    expect(resolutions.single.oursId, 'o');
    expect(resolutions.single.theirsId, 't');
    expect(resolutions.single.content, 'bread\nchai\n');
  });

  test('a note without a finished draft stops the merge', () async {
    final detail = _detail('a.pn');
    final unfinished = freshDraft(detail)
        .copyWith(choices: {1: const Choice(Pick.ours)});

    expect(
      () => collectResolutions(_pending(['a.pn']), {
        'a.pn': unfinished,
      }, (_) async => detail),
      throwsStateError,
    );
    expect(
      () => collectResolutions(_pending(['a.pn']), {}, (_) async => detail),
      throwsStateError,
    );
  });

  test('a draft made against an older version stops the merge', () async {
    final draft = freshDraft(_detail('a.pn'))
        .copyWith(choices: {1: const Choice(Pick.ours)}, done: true);

    expect(
      () => collectResolutions(_pending(['a.pn']), {
        'a.pn': draft,
      }, (_) async => _detail('a.pn', oursId: 'o2')),
      throwsStateError,
    );
  });
}
