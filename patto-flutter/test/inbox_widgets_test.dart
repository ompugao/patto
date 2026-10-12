import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/inbox/widgets/inbox_composer.dart';
import 'package:patto_flutter/features/inbox/widgets/inbox_posts.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

InboxPost _post(String date, String text, {List<String> body = const []}) =>
    InboxPost(date: date, time: '09:00', text: text, body: body, line: 0);

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  group('InboxPostList', () {
    testWidgets('groups posts under one heading per day', (tester) async {
      await tester.pumpWidget(
        _host(
          InboxPostList(
            posts: [
              _post('2020-01-01', 'first'),
              _post('2020-01-01', 'second', body: ['detail']),
              _post('2020-01-02', 'third'),
            ],
            controller: ScrollController(),
            onTap: (_) {},
          ),
        ),
      );

      expect(find.text('Wed, 1 Jan 2020'), findsOneWidget);
      expect(find.text('Thu, 2 Jan 2020'), findsOneWidget);
      expect(find.text('detail'), findsOneWidget);
    });

    testWidgets('tapping a post hands it back', (tester) async {
      InboxPost? tapped;
      await tester.pumpWidget(
        _host(
          InboxPostList(
            posts: [_post('2020-01-01', 'first')],
            controller: ScrollController(),
            onTap: (post) => tapped = post,
          ),
        ),
      );

      await tester.tap(find.text('first'));
      expect(tapped?.text, 'first');
    });

    testWidgets('says what the inbox is for while it is empty', (tester) async {
      await tester.pumpWidget(
        _host(
          InboxPostList(
            posts: const [],
            controller: ScrollController(),
            onTap: (_) {},
          ),
        ),
      );

      expect(find.textContaining('Nothing here yet'), findsOneWidget);
    });
  });

  group('InboxComposer', () {
    testWidgets(
      'the send button is disabled until there is something to send',
      (tester) async {
        var sent = 0;
        Widget composer({required bool canSend}) => _host(
          InboxComposer(
            controller: TextEditingController(),
            focusNode: FocusNode(),
            canSend: canSend,
            sending: false,
            onChanged: () {},
            onSend: () => sent++,
          ),
        );

        await tester.pumpWidget(composer(canSend: false));
        await tester.tap(find.byTooltip('Post'));
        expect(sent, 0);

        await tester.pumpWidget(composer(canSend: true));
        await tester.tap(find.byTooltip('Post'));
        expect(sent, 1);
      },
    );

    testWidgets('a spinner replaces the button while sending', (tester) async {
      await tester.pumpWidget(
        _host(
          InboxComposer(
            controller: TextEditingController(),
            focusNode: FocusNode(),
            canSend: false,
            sending: true,
            onChanged: () {},
            onSend: () {},
          ),
        ),
      );

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.byIcon(Icons.send), findsNothing);
    });
  });
}
