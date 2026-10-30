import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/widgets/code_block.dart';
import 'package:patto_flutter/features/notes/widgets/joined_selection.dart';
import 'package:patto_flutter/features/notes/widgets/spans_text.dart';
import 'package:patto_flutter/features/notes/widgets/table_block.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

NoteTableRow _row(List<String> cells) => NoteTableRow(
  cells: [
    for (final c in cells) NoteTableCell(spans: [NoteSpan.text(text: c)]),
  ],
);

void main() {
  late List<String?> reports;

  setUp(() => reports = []);

  Future<void> selectAll(WidgetTester tester, Widget child) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SelectionArea(
            child: JoinedSelection(onChanged: reports.add, child: child),
          ),
        ),
      ),
    );
    // Text inside a scroll view registers for selection a frame late.
    await tester.pump();
    await tester.pump();
    tester
        .state<SelectableRegionState>(find.byType(SelectableRegion))
        .selectAll();
    await tester.pump();
  }

  testWidgets('reports mounting, and no selection until there is one', (
    tester,
  ) async {
    var mounted = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: SelectionArea(
          child: JoinedSelection(
            onChanged: reports.add,
            onMounted: () => mounted++,
            child: const Text('a'),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(mounted, 1);
    expect(reports, isEmpty);
  });

  testWidgets('reports null once a selection is cleared', (tester) async {
    await selectAll(tester, const Text('a'));
    tester
        .state<SelectableRegionState>(find.byType(SelectableRegion))
        .clearSelection();
    await tester.pump();

    expect(reports, ['a', null]);
  });

  testWidgets('parts on separate rows are joined with a newline', (
    tester,
  ) async {
    await selectAll(
      tester,
      const Column(children: [Text('first'), Text('second')]),
    );

    expect(reports.last, 'first\nsecond');
  });

  testWidgets('table cells are joined with tabs, rows with newlines', (
    tester,
  ) async {
    await selectAll(
      tester,
      TableBlock(
        caption: null,
        rows: [
          _row(['a', 'b']),
          _row(['c', 'd']),
        ],
        actions: SpanActions(
          onWikiLink: (_, _) {},
          onUrl: (_) {},
          onAnchor: (_) {},
          onEmbed: (_) {},
        ),
        root: null,
        textScale: 1,
      ),
    );

    expect(reports.last, 'a\tb\nc\td');
  });

  testWidgets('a code block copies its code without the language label', (
    tester,
  ) async {
    await selectAll(
      tester,
      const CodeBlock(lang: 'dart', lines: ['void main() {}'], textScale: 1),
    );

    expect(reports.last, 'void main() {}');
  });
}
