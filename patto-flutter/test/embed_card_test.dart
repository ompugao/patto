import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/widgets/embed_card.dart';
import 'package:patto_flutter/features/notes/widgets/embed_card_parts.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  group('EmbedCard', () {
    testWidgets('a PDF without a title is named after its file', (
      tester,
    ) async {
      await tester.pumpWidget(
        _host(
          EmbedCard(
            embed: const EmbedRef(
              url: './papers/Attention.pdf',
              kind: EmbedKind.pdf(),
              isLocal: true,
            ),
            onTap: () {},
          ),
        ),
      );

      expect(find.text('Attention.pdf'), findsOneWidget);
      expect(find.text('PDF'), findsOneWidget);
    });

    testWidgets('a titled PDF keeps the file name in the subtitle', (
      tester,
    ) async {
      var taps = 0;
      await tester.pumpWidget(
        _host(
          EmbedCard(
            embed: const EmbedRef(
              url: 'https://example.com/a.pdf',
              title: 'Paper',
              kind: EmbedKind.pdf(),
              isLocal: false,
            ),
            onTap: () => taps++,
          ),
        ),
      );

      expect(find.text('Paper'), findsOneWidget);
      expect(find.text('PDF · a.pdf'), findsOneWidget);
      await tester.tap(find.text('Paper'));
      expect(taps, 1);
    });
  });

  group('LookupBuilder', () {
    testWidgets('looks a URL up once and again only when it changes', (
      tester,
    ) async {
      final looked = <String>[];
      Future<String?> lookup(String url) async {
        looked.add(url);
        return 'result for $url';
      }

      Widget card(String url) => _host(
        LookupBuilder<String>(
          url: url,
          lookup: lookup,
          builder: (context, snapshot) => Text(snapshot.data ?? 'loading'),
        ),
      );

      await tester.pumpWidget(card('a'));
      expect(find.text('loading'), findsOneWidget);
      await tester.pump();
      expect(find.text('result for a'), findsOneWidget);

      await tester.pumpWidget(card('a'));
      await tester.pump();
      expect(looked, ['a']);

      await tester.pumpWidget(card('b'));
      await tester.pump();
      expect(looked, ['a', 'b']);
      expect(find.text('result for b'), findsOneWidget);
    });
  });

  testWidgets('FixedLines reserves the height of its line count', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            FixedLines(
              '',
              lines: 2,
              style: TextStyle(fontSize: 10, height: 1.5),
              textScale: 1,
            ),
          ],
        ),
      ),
    );

    expect(tester.getSize(find.byType(FixedLines)).height, 30);
  });
}
