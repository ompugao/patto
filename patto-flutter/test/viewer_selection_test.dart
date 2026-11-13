import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/viewer_selection.dart';
import 'package:patto_flutter/features/notes/widgets/joined_selection.dart';

void main() {
  bool allText(int _) => true;

  group('model', () {
    late ViewerSelection selection;

    setUp(() => selection = ViewerSelection());

    test('stays active when its blocks scroll out of the list', () {
      selection
        ..blockChanged(1, 'a')
        ..regionChanged(hasText: true)
        ..regionChanged(hasText: false);

      expect(selection.active, isTrue);
      expect(selection.texts, {1: 'a'});
    });

    test('ends when Flutter clears it', () {
      selection
        ..blockChanged(1, 'a')
        ..regionChanged(hasText: true)
        ..blockChanged(1, null)
        ..regionChanged(hasText: false);

      expect(selection.active, isFalse);
      expect(selection.range(5, allText), isNull);
    });

    test('a clear also drops blocks that were out of the list', () {
      selection
        ..blockChanged(1, 'a')
        ..blockChanged(2, 'b')
        ..regionChanged(hasText: true)
        ..blockChanged(2, null)
        ..regionChanged(hasText: false);

      expect(selection.texts, isEmpty);
    });

    test('Select all covers every block until a block changes', () {
      selection
        ..blockChanged(0, 'a')
        ..selectAll();
      expect(selection.range(5, allText), (0, 4));
      expect(selection.texts, isNull);

      selection.blockChanged(0, 'a');
      expect(selection.texts, isNull);

      selection.blockChanged(0, null);
      expect(selection.texts, isEmpty);
    });

    test('a rebuilt block loses its text until Flutter re-selects it', () {
      selection
        ..blockChanged(1, 'a')
        ..blockMounted(1);

      expect(selection.texts, isEmpty);
    });
  });

  group('with Flutter selection', () {
    late ViewerSelection selection;

    setUp(() => selection = ViewerSelection());

    Future<SelectableRegionState> pumpList(
      WidgetTester tester,
      ScrollController controller,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SelectionArea(
              onSelectionChanged: (content) => selection.regionChanged(
                hasText: content?.plainText.isNotEmpty ?? false,
              ),
              child: ListView.builder(
                controller: controller,
                itemCount: 200,
                itemExtent: 40,
                itemBuilder: (_, i) => JoinedSelection(
                  onChanged: (text) => selection.blockChanged(i, text),
                  onMounted: () => selection.blockMounted(i),
                  child: Text('line $i'),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      final region = tester.state<SelectableRegionState>(
        find.byType(SelectableRegion),
      );
      await tester.longPress(find.text('line 1'));
      await tester.pump();
      return region;
    }

    testWidgets('a selection scrolled out of the list stays active', (
      tester,
    ) async {
      final controller = ScrollController();
      await pumpList(tester, controller);
      expect(selection.active, isTrue);

      controller.jumpTo(4000);
      await tester.pumpAndSettle();

      expect(find.text('line 1'), findsNothing);
      expect(selection.active, isTrue);
      expect(selection.range(200, allText), (1, 1));
    });

    testWidgets('tapping the text ends the selection', (tester) async {
      await pumpList(tester, ScrollController());

      await tester.tap(find.text('line 3'));
      await tester.pumpAndSettle();

      expect(selection.active, isFalse);
    });

    testWidgets('clearing the selection ends it', (tester) async {
      final region = await pumpList(tester, ScrollController());

      region.clearSelection();
      await tester.pump();

      expect(selection.active, isFalse);
    });
  });
}
