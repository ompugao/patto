import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/trackpad.dart';
import 'package:patto_flutter/features/editor/widgets/editor_toolbar.dart';
import 'package:patto_flutter/features/editor/widgets/link_candidate_bar.dart';
import 'package:patto_flutter/src/rust/api/types.dart';
import 'package:re_editor/re_editor.dart';

NoteMeta _note(String name) => NoteMeta(
  name: name,
  relPath: '$name.pn',
  modifiedMs: 0,
  sizeBytes: BigInt.zero,
);

void main() {
  group('EditorToolbar', () {
    final inserted = <(String, int)>[];
    final moves = <bool>[];
    var indents = 0;

    Future<void> pump(WidgetTester tester) async {
      inserted.clear();
      moves.clear();
      indents = 0;
      // The bar scrolls sideways and builds only what is visible; a wide
      // surface keeps every button on screen.
      tester.view.physicalSize = const Size(2000, 400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final controller = CodeLineEditingController.fromText('');
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: EditorToolbar(
              trackpad: CaretTrackpad(
                controller: controller,
                paragraphs: () => null,
              ),
              onIndent: () => indents++,
              onOutdent: () => indents--,
              onInsert: (text, [back = 0]) => inserted.add((text, back)),
              onAttach: () {},
              onPaste: () {},
              onMoveBlock: moves.add,
              onSelectBlock: () {},
              onUndo: () {},
              onRedo: () {},
              today: () => '2026-10-09',
            ),
          ),
        ),
      );
    }

    testWidgets('the snippet buttons insert patto markup', (tester) async {
      await pump(tester);
      for (final label in ['[ ]', 'task', 'due', 'code', 'quote']) {
        await tester.tap(find.text(label));
      }
      expect(inserted, [
        ('[]', 1),
        ('{@task status=todo}', 0),
        ('!2026-10-09', 0),
        ('[@code ]', 1),
        ('[@quote]', 0),
      ]);
    });

    testWidgets('the block buttons indent and move', (tester) async {
      await pump(tester);
      await tester.tap(find.byTooltip('Indent block'));
      expect(indents, 1);
      await tester.tap(find.byTooltip('Outdent block'));
      expect(indents, 0);
      await tester.tap(find.byTooltip('Move block up'));
      await tester.tap(find.byTooltip('Move block down'));
      expect(moves, [true, false]);
    });
  });

  group('LinkCandidateBar', () {
    testWidgets('offers the typed name as a new note when none matches', (
      tester,
    ) async {
      String? picked;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LinkCandidateBar(
              candidates: [_note('project'), _note('projects')],
              query: 'proj',
              onPick: (name) => picked = name,
            ),
          ),
        ),
      );

      expect(find.text('proj'), findsOneWidget);
      expect(find.byIcon(Icons.add), findsOneWidget);
      await tester.tap(find.text('projects'));
      expect(picked, 'projects');
    });

    testWidgets('does not offer a new note that already exists', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LinkCandidateBar(
              candidates: [_note('project')],
              query: 'project',
              onPick: (_) {},
            ),
          ),
        ),
      );

      expect(find.text('project'), findsOneWidget);
      expect(find.byIcon(Icons.add), findsNothing);
    });
  });
}
