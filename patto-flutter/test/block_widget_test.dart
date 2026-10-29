import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/widgets/block_widget.dart';
import 'package:patto_flutter/features/notes/widgets/spans_text.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

Block _block(BlockKind kind) =>
    Block(row: 3, depth: 0, quoteDepth: 0, anchors: const [], kind: kind);

SpanActions _actions({void Function(String, String?)? onWikiLink}) =>
    SpanActions(
      onWikiLink: onWikiLink ?? (_, _) {},
      onUrl: (_) {},
      onAnchor: (_) {},
      onEmbed: (_) {},
    );

void main() {
  late List<String> events;

  setUp(() => events = []);

  Widget host(Block block, {SpanActions? actions}) => MaterialApp(
    home: Scaffold(
      body: SelectionArea(
        child: ListView(
          children: [
            BlockWidget(
              block: block,
              actions: actions ?? _actions(),
              root: null,
              onLongPress: (b) => events.add('long-press ${b.row}'),
            ),
          ],
        ),
      ),
    ),
  );

  testWidgets('long-press on text selects it instead of opening the editor', (
    tester,
  ) async {
    await tester.pumpWidget(
      host(
        _block(
          const BlockKind.line(spans: [NoteSpan.text(text: 'hello world')]),
        ),
      ),
    );

    // A scroll view registers its selectable children a frame after layout.
    await tester.pump();
    await tester.longPressAt(
      tester.getTopLeft(find.textContaining('hello')) + const Offset(8, 8),
    );
    await tester.pumpAndSettle();

    expect(events, isEmpty);
    expect(find.text('Copy'), findsOneWidget);
  });

  testWidgets('a tap on a link still follows it', (tester) async {
    await tester.pumpWidget(
      host(
        _block(const BlockKind.line(spans: [NoteSpan.wikiLink(name: 'other')])),
        actions: _actions(onWikiLink: (name, _) => events.add('link $name')),
      ),
    );

    await tester.tapOnText(find.textRange.ofSubstring('other'));
    await tester.pumpAndSettle();

    expect(events, ['link other']);
  });

  testWidgets('long-press on a block without text opens the editor', (
    tester,
  ) async {
    await tester.pumpWidget(host(_block(const BlockKind.rule())));

    await tester.longPress(find.byType(Divider));
    await tester.pumpAndSettle();

    expect(events, ['long-press 3']);
  });
}
