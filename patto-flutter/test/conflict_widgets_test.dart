import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/conflicts/conflict_state.dart';
import 'package:patto_flutter/features/conflicts/widgets/conflict_card.dart';
import 'package:patto_flutter/features/conflicts/widgets/merge_lines.dart';
import 'package:patto_flutter/features/conflicts/widgets/whole_note_choice.dart';
import 'package:patto_flutter/src/rust/api/conflict.dart';
import 'package:patto_flutter/src/rust/api/merge.dart';

const _conflict = MergeRegion_Conflict(
  base: ['tea'],
  ours: ['green tea'],
  theirs: ['black tea'],
  suggestion: Suggestion(kind: SuggestionKind.both, lines: []),
);

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: ListView(children: [child])),
);

void main() {
  group('ConflictCard', () {
    testWidgets('shows both sides until a choice is made', (tester) async {
      await tester.pumpWidget(
        _host(
          ConflictCard(
            region: _conflict,
            choice: null,
            onChoose: (_) {},
            onEdit: () {},
          ),
        ),
      );

      expect(find.text('Both changed this'), findsOneWidget);
      expect(find.text('green tea'), findsOneWidget);
      expect(find.text('black tea'), findsOneWidget);
      expect(find.text('✦ Both'), findsOneWidget);
    });

    testWidgets('tapping a side reports that choice', (tester) async {
      Choice? chosen;
      await tester.pumpWidget(
        _host(
          ConflictCard(
            region: _conflict,
            choice: null,
            onChoose: (c) => chosen = c,
            onEdit: () {},
          ),
        ),
      );

      await tester.tap(find.widgetWithText(ChoiceChip, 'Remote'));
      expect(chosen?.pick, Pick.theirs);
    });

    testWidgets('tapping the chosen side again clears the choice', (
      tester,
    ) async {
      Choice? chosen = const Choice(Pick.ours);
      await tester.pumpWidget(
        _host(
          ConflictCard(
            region: _conflict,
            choice: chosen,
            onChoose: (c) => chosen = c,
            onEdit: () {},
          ),
        ),
      );

      await tester.tap(find.widgetWithText(ChoiceChip, 'You'));
      expect(chosen, isNull);
    });

    testWidgets('a settled card shows only the chosen lines', (tester) async {
      await tester.pumpWidget(
        _host(
          ConflictCard(
            region: _conflict,
            choice: const Choice(Pick.theirs),
            onChoose: (_) {},
            onEdit: () {},
          ),
        ),
      );

      expect(find.text('Using remote'), findsOneWidget);
      expect(find.text('black tea'), findsOneWidget);
      expect(find.text('green tea'), findsNothing);

      await tester.tap(find.text('compare'));
      await tester.pump();
      expect(find.text('green tea'), findsOneWidget);
      expect(find.text('hide sides'), findsOneWidget);
    });

    testWidgets('the edit chip asks for lines by hand', (tester) async {
      var edits = 0;
      await tester.pumpWidget(
        _host(
          ConflictCard(
            region: _conflict,
            choice: null,
            onChoose: (_) {},
            onEdit: () => edits++,
          ),
        ),
      );

      await tester.tap(find.text('Edit'));
      expect(edits, 1);
    });
  });

  group('UnchangedLines', () {
    const lines = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];

    testWidgets('a long run collapses to its edges until tapped', (
      tester,
    ) async {
      var expanded = false;
      await tester.pumpWidget(
        _host(
          UnchangedLines(
            lines: lines,
            expanded: false,
            onExpand: () => expanded = true,
          ),
        ),
      );

      expect(find.text('⋯ 3 unchanged lines'), findsOneWidget);
      expect(find.text('d'), findsNothing);
      await tester.tap(find.text('⋯ 3 unchanged lines'));
      expect(expanded, isTrue);
    });

    testWidgets('a short run is shown whole', (tester) async {
      await tester.pumpWidget(
        _host(
          UnchangedLines(
            lines: lines.take(5).toList(),
            expanded: false,
            onExpand: () {},
          ),
        ),
      );

      expect(find.textContaining('unchanged lines'), findsNothing);
      expect(find.text('c'), findsOneWidget);
    });
  });

  group('OneSidedLines', () {
    testWidgets('an undone change shows the lines it replaced', (tester) async {
      await tester.pumpWidget(
        _host(
          const OneSidedLines(
            side: MergeSide.ours,
            base: ['old'],
            lines: ['new'],
            undone: true,
          ),
        ),
      );

      expect(find.text('old'), findsOneWidget);
      expect(find.text('new'), findsNothing);
      expect(find.text('you · undone'), findsOneWidget);
    });

    testWidgets('a deletion is shown struck through with its tag', (
      tester,
    ) async {
      await tester.pumpWidget(
        _host(
          const OneSidedLines(
            side: MergeSide.theirs,
            base: ['gone'],
            lines: [],
            undone: false,
          ),
        ),
      );

      expect(find.text('gone'), findsOneWidget);
      expect(find.text('remote deleted'), findsOneWidget);
    });
  });

  group('WholeNoteChoice', () {
    ConflictDetail detail(ConflictKind kind) => ConflictDetail(
      path: 'n.pn',
      kind: kind,
      oursId: kind == ConflictKind.deletedByUs ? null : 'o',
      theirsId: kind == ConflictKind.deletedByThem ? null : 't',
      ours: kind == ConflictKind.deletedByUs ? null : 'mine',
      theirs: kind == ConflictKind.deletedByThem ? null : 'theirs',
      merged: const MergedNote(regions: [], trailingNewline: true),
    );

    testWidgets('keeping a note the remote deleted picks our side', (
      tester,
    ) async {
      Choice? chosen;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: WholeNoteChoice(
              detail: detail(ConflictKind.deletedByThem),
              choice: null,
              onChoose: (c) => chosen = c,
            ),
          ),
        ),
      );

      expect(find.text('Your version'), findsOneWidget);
      await tester.tap(find.text('Keep it'));
      expect(chosen?.pick, Pick.ours);
    });

    testWidgets('restoring a note we deleted picks the remote side', (
      tester,
    ) async {
      Choice? chosen;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: WholeNoteChoice(
              detail: detail(ConflictKind.deletedByUs),
              choice: null,
              onChoose: (c) => chosen = c,
            ),
          ),
        ),
      );

      await tester.tap(find.text('Restore it'));
      expect(chosen?.pick, Pick.theirs);
      await tester.tap(find.text('Delete it'));
      expect(chosen?.pick, Pick.ours);
    });
  });
}
